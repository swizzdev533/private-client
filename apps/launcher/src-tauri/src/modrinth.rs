use crate::contracts::{
    ModCompatibility, ModEnvironment, ModInstallPlan, ModInstallPlanItem, ModSearchRequest,
    ModSearchResponse, ModSearchSort, ModSummary, ModTrust, ReleaseType,
};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::{validate_identifier, JAR_LIMIT};
use crate::network::{validate_url_text, SecureHttpClient};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use url::Url;

const SEARCH_ENDPOINT: &str = "https://api.modrinth.com/v2/search";
const API_ROOT: &str = "https://api.modrinth.com/v2";
const MAX_DEPENDENCIES: usize = 64;

#[derive(Debug, Clone)]
pub struct ResolvedInstallPlan {
    pub public: ModInstallPlan,
    pub nodes: Vec<ResolvedVersion>,
}

#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub project_id: String,
    pub project_name: String,
    pub version_id: String,
    pub version_number: String,
    pub file: VersionFile,
    pub required_dependency: bool,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionFile {
    pub hashes: BTreeMap<String, String>,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub version_type: String,
    pub date_published: DateTime<Utc>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<VersionDependency>,
}

#[derive(Debug, Deserialize)]
struct SearchApiResponse {
    hits: Vec<SearchApiHit>,
    offset: u32,
    limit: u32,
    total_hits: u32,
}

#[derive(Debug, Deserialize)]
struct SearchApiHit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    author: String,
    icon_url: Option<String>,
    downloads: u64,
    latest_version: Option<String>,
    license: Option<String>,
    date_modified: DateTime<Utc>,
    client_side: Option<String>,
    server_side: Option<String>,
    #[serde(default)]
    display_categories: Vec<String>,
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Project {
    id: String,
    title: String,
}

pub async fn search(
    network: &SecureHttpClient,
    request: &ModSearchRequest,
) -> AppResult<ModSearchResponse> {
    validate_search(request)?;
    let page_size = 20_u32;
    let offset = request.page.saturating_mul(page_size);
    let index = match request.sort {
        ModSearchSort::Relevance => "relevance",
        ModSearchSort::Downloads => "downloads",
        ModSearchSort::Updated => "updated",
    };
    let _ = &request.trust;
    let facets = serde_json::to_string(&[
        ["project_type:mod"],
        ["categories:forge"],
        ["versions:1.8.9"],
    ])
    .map_err(|error| AppError::json("Could not encode Modrinth facets", error))?;
    let mut url = Url::parse(SEARCH_ENDPOINT)?;
    url.query_pairs_mut()
        .append_pair("query", request.query.trim())
        .append_pair("offset", &offset.to_string())
        .append_pair("limit", &page_size.to_string())
        .append_pair("index", index)
        .append_pair("facets", &facets);
    let response: SearchApiResponse = network.get_json(url.as_str()).await?;
    let results = response
        .hits
        .into_iter()
        .map(|hit| {
            let license = hit.license.unwrap_or_else(|| "Unknown".to_owned());
            let license_unknown = license.eq_ignore_ascii_case("unknown")
                || license.eq_ignore_ascii_case("arr")
                || license.trim().is_empty();
            let compatible = hit.latest_version.is_some();
            let compatibility = if !compatible {
                ModCompatibility::DownloadUnavailable
            } else if license_unknown {
                ModCompatibility::LicenseReview
            } else {
                ModCompatibility::Compatible
            };
            let environment = if hit.server_side.as_deref() == Some("required") {
                ModEnvironment::ClientAndServer
            } else {
                ModEnvironment::Client
            };
            let _ = (&hit.display_categories, &hit.categories, &hit.client_side);
            ModSummary {
                id: hit.slug,
                project_id: hit.project_id,
                version_id: hit
                    .latest_version
                    .clone()
                    .unwrap_or_else(|| "unavailable".to_owned()),
                name: hit.title,
                description: hit.description,
                author: hit.author,
                icon_url: hit.icon_url,
                version: hit
                    .latest_version
                    .unwrap_or_else(|| "Unavailable".to_owned()),
                release_type: ReleaseType::Release,
                downloads: hit.downloads,
                updated_at: hit.date_modified.to_rfc3339(),
                minecraft_version: crate::minecraft::MINECRAFT_VERSION.to_owned(),
                loader: "forge".to_owned(),
                environment,
                license,
                file_size: 0,
                dependency_count: 0,
                trust: ModTrust::FromModrinth,
                compatibility,
                compatibility_reason: if license_unknown {
                    Some("Project license requires manual review.".to_owned())
                } else if !compatible {
                    Some("No compatible Forge 1.8.9 download is available.".to_owned())
                } else {
                    None
                },
                installed: false,
                installed_version: None,
                update_available: false,
                required: false,
            }
        })
        .collect();
    Ok(ModSearchResponse {
        query: request.query.clone(),
        results,
        page: request.page,
        has_more: response.offset.saturating_add(response.limit) < response.total_hits,
        from_cache: false,
        offline: false,
    })
}

pub async fn resolve_plan(
    network: &SecureHttpClient,
    project_id: &str,
    version_id: Option<&str>,
    allow_beta: bool,
) -> AppResult<ResolvedInstallPlan> {
    validate_identifier(project_id, "Modrinth project ID")?;
    let root = if let Some(version_id) = version_id {
        validate_identifier(version_id, "Modrinth version ID")?;
        let version = get_version(network, version_id).await?;
        if version.project_id != project_id {
            return Err(AppError::new(
                AppErrorCode::ModIncompatible,
                "The selected version does not belong to the requested project",
            ));
        }
        validate_compatible_version(&version, allow_beta)?;
        version
    } else {
        select_version(network, project_id, allow_beta).await?
    };
    let mut queue = VecDeque::from([(root, false)]);
    let mut versions = BTreeMap::<String, ProjectVersion>::new();
    let mut required_flags = BTreeMap::<String, bool>::new();
    let mut graph = BTreeMap::<String, Vec<String>>::new();
    let mut warnings = Vec::new();

    while let Some((version, required_dependency)) = queue.pop_front() {
        if versions.len() >= MAX_DEPENDENCIES {
            return Err(AppError::new(
                AppErrorCode::DependencyConflict,
                "The dependency graph exceeds the safe limit",
            ));
        }
        required_flags
            .entry(version.project_id.clone())
            .and_modify(|required| *required |= required_dependency)
            .or_insert(required_dependency);
        if versions.contains_key(&version.project_id) {
            continue;
        }
        let mut edges = Vec::new();
        for dependency in &version.dependencies {
            match dependency.dependency_type.as_str() {
                "required" => {
                    let dependency_version = if let Some(id) = dependency.version_id.as_deref() {
                        get_version(network, id).await?
                    } else if let Some(id) = dependency.project_id.as_deref() {
                        select_version(network, id, allow_beta).await?
                    } else {
                        return Err(AppError::new(
                            AppErrorCode::DependencyConflict,
                            "A required Modrinth dependency has no project or version ID",
                        ));
                    };
                    validate_compatible_version(&dependency_version, allow_beta)?;
                    edges.push(dependency_version.project_id.clone());
                    queue.push_back((dependency_version, true));
                }
                "optional" => warnings.push(format!(
                    "{} has an optional dependency that was not installed automatically",
                    version.name
                )),
                "incompatible" => {
                    if let Some(project) = dependency.project_id.as_deref() {
                        warnings.push(format!(
                            "{} declares project {} as incompatible",
                            version.name, project
                        ));
                    }
                }
                _ => {}
            }
        }
        graph.insert(version.project_id.clone(), edges);
        versions.insert(version.project_id.clone(), version);
    }
    detect_cycles(&graph)?;

    let mut nodes = Vec::new();
    for (resolved_project_id, version) in versions {
        let project = get_project(network, &resolved_project_id).await?;
        let file = choose_file(&version)?;
        let dependencies = graph.get(&resolved_project_id).cloned().unwrap_or_default();
        nodes.push(ResolvedVersion {
            project_id: project.id,
            project_name: project.title,
            version_id: version.id,
            version_number: version.version_number,
            file,
            required_dependency: required_flags
                .get(&resolved_project_id)
                .copied()
                .unwrap_or(false),
            dependencies,
        });
    }
    nodes.sort_by_key(|node| !node.required_dependency);
    let total_download_bytes = nodes
        .iter()
        .fold(0_u64, |total, node| total.saturating_add(node.file.size));
    let items: Vec<ModInstallPlanItem> = nodes
        .iter()
        .map(|node| ModInstallPlanItem {
            project_id: node.project_id.clone(),
            version_id: node.version_id.clone(),
            name: node.project_name.clone(),
            version: node.version_number.clone(),
            file_size: node.file.size,
            required: node.required_dependency,
        })
        .collect();
    let requested_mod = items
        .iter()
        .find(|item| item.project_id == project_id)
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::DependencyConflict,
                "The resolved plan does not contain the requested project",
            )
        })?;
    let dependencies = items
        .into_iter()
        .filter(|item| item.project_id != project_id)
        .collect();
    Ok(ResolvedInstallPlan {
        public: ModInstallPlan {
            requested_mod,
            dependencies,
            expected_disk_usage: total_download_bytes,
            files_to_replace: Vec::new(),
            warnings,
        },
        nodes,
    })
}

async fn get_project(network: &SecureHttpClient, project_id: &str) -> AppResult<Project> {
    validate_identifier(project_id, "Modrinth project ID")?;
    network
        .get_json(&format!("{API_ROOT}/project/{project_id}"))
        .await
        .map_err(|error| {
            AppError::new(
                AppErrorCode::ModNotFound,
                "The Modrinth project was not found",
            )
            .details(error.to_string())
        })
}

async fn get_version(network: &SecureHttpClient, version_id: &str) -> AppResult<ProjectVersion> {
    validate_identifier(version_id, "Modrinth version ID")?;
    network
        .get_json(&format!("{API_ROOT}/version/{version_id}"))
        .await
        .map_err(|error| {
            AppError::new(
                AppErrorCode::ModNotFound,
                "The Modrinth version was not found",
            )
            .details(error.to_string())
        })
}

async fn select_version(
    network: &SecureHttpClient,
    project_id: &str,
    allow_beta: bool,
) -> AppResult<ProjectVersion> {
    validate_identifier(project_id, "Modrinth project ID")?;
    let mut url = Url::parse(&format!("{API_ROOT}/project/{project_id}/version"))?;
    let game_versions = serde_json::to_string(&[crate::minecraft::MINECRAFT_VERSION])
        .map_err(|error| AppError::json("Could not encode game version filter", error))?;
    let loaders = serde_json::to_string(&["forge"])
        .map_err(|error| AppError::json("Could not encode loader filter", error))?;
    url.query_pairs_mut()
        .append_pair("game_versions", &game_versions)
        .append_pair("loaders", &loaders);
    let mut versions: Vec<ProjectVersion> = network.get_json(url.as_str()).await?;
    versions.retain(|version| validate_compatible_version(version, allow_beta).is_ok());
    versions.sort_by(|left, right| {
        version_type_rank(&left.version_type)
            .cmp(&version_type_rank(&right.version_type))
            .then_with(|| right.date_published.cmp(&left.date_published))
    });
    versions.into_iter().next().ok_or_else(|| {
        AppError::new(
            AppErrorCode::ModIncompatible,
            "No compatible Forge 1.8.9 release is available",
        )
    })
}

fn validate_compatible_version(version: &ProjectVersion, allow_beta: bool) -> AppResult<()> {
    let supports_game = version
        .game_versions
        .iter()
        .any(|game| game == crate::minecraft::MINECRAFT_VERSION);
    let supports_forge = version
        .loaders
        .iter()
        .any(|loader| loader.eq_ignore_ascii_case("forge"));
    let channel_allowed =
        version.version_type == "release" || allow_beta && version.version_type == "beta";
    if supports_game && supports_forge && channel_allowed {
        Ok(())
    } else {
        Err(AppError::new(
            AppErrorCode::ModIncompatible,
            "The selected mod version is not an allowed Forge 1.8.9 release",
        ))
    }
}

fn choose_file(version: &ProjectVersion) -> AppResult<VersionFile> {
    let file = version
        .files
        .iter()
        .find(|file| file.primary && file.filename.to_ascii_lowercase().ends_with(".jar"))
        .or_else(|| {
            version
                .files
                .iter()
                .find(|file| file.filename.to_ascii_lowercase().ends_with(".jar"))
        })
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::ModIncompatible,
                "The selected version has no JAR file",
            )
        })?;
    if file.size == 0 || file.size > JAR_LIMIT {
        return Err(AppError::new(
            AppErrorCode::DownloadTooLarge,
            "The selected mod file has an invalid size",
        ));
    }
    validate_file_name(&file.filename)?;
    let url = validate_url_text(&file.url)?;
    if url.host_str() != Some("cdn.modrinth.com") {
        return Err(AppError::new(
            AppErrorCode::UntrustedHost,
            "Mod files must be served by the official Modrinth CDN",
        ));
    }
    let sha512 = file.hashes.get("sha512").ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "Modrinth did not provide a SHA-512 hash for the selected file",
        )
    })?;
    if sha512.len() != 128 || !sha512.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "Modrinth returned an invalid SHA-512 hash",
        ));
    }
    Ok(file)
}

fn validate_file_name(value: &str) -> AppResult<()> {
    if value.len() > 180
        || value.is_empty()
        || value.contains(['/', '\\', ':'])
        || value == "."
        || value == ".."
    {
        return Err(AppError::new(
            AppErrorCode::PathTraversalDetected,
            "A provider supplied an unsafe file name",
        ));
    }
    Ok(())
}

fn validate_search(request: &ModSearchRequest) -> AppResult<()> {
    let query = request.query.trim();
    if query.len() > 120 || query.chars().any(char::is_control) {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "The Modrinth search query is invalid",
        ));
    }
    if request.page > 500 {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "The Modrinth pagination values are outside the safe range",
        ));
    }
    Ok(())
}

fn version_type_rank(value: &str) -> u8 {
    match value {
        "release" => 0,
        "beta" => 1,
        _ => 2,
    }
}

fn detect_cycles(graph: &BTreeMap<String, Vec<String>>) -> AppResult<()> {
    fn visit(
        node: &str,
        graph: &BTreeMap<String, Vec<String>>,
        visiting: &mut BTreeSet<String>,
        complete: &mut BTreeSet<String>,
    ) -> AppResult<()> {
        if complete.contains(node) {
            return Ok(());
        }
        if !visiting.insert(node.to_owned()) {
            return Err(AppError::new(
                AppErrorCode::DependencyCycle,
                "A required mod dependency cycle was detected",
            )
            .details(node.to_owned()));
        }
        if let Some(edges) = graph.get(node) {
            for edge in edges {
                visit(edge, graph, visiting, complete)?;
            }
        }
        visiting.remove(node);
        complete.insert(node.to_owned());
        Ok(())
    }

    let mut complete = BTreeSet::new();
    for node in graph.keys() {
        visit(node, graph, &mut BTreeSet::new(), &mut complete)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{detect_cycles, validate_file_name, version_type_rank};
    use std::collections::BTreeMap;

    #[test]
    fn release_is_preferred_to_beta() {
        assert!(version_type_rank("release") < version_type_rank("beta"));
        assert!(version_type_rank("beta") < version_type_rank("alpha"));
    }

    #[test]
    fn blocks_unsafe_provider_file_names() {
        assert!(validate_file_name("safe-mod.jar").is_ok());
        assert!(validate_file_name("../escape.jar").is_err());
        assert!(validate_file_name("nested/mod.jar").is_err());
    }

    #[test]
    fn detects_dependency_cycles() {
        let mut graph = BTreeMap::new();
        graph.insert("a".to_owned(), vec!["b".to_owned()]);
        graph.insert("b".to_owned(), vec!["a".to_owned()]);
        assert!(detect_cycles(&graph).is_err());
    }
}
