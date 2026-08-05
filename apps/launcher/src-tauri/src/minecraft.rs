use crate::contracts::{InstanceSnapshot, ProgressEvent, EVENT_OPERATION_PROGRESS};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::{
    atomic_write, atomic_write_json, extract_zip_safely, hash_bytes, hash_file, replace_file,
    safe_relative_path, validate_identifier, validate_jar, JAR_LIMIT,
};
use crate::logging::LocalLogger;
use crate::network::{DownloadExpectation, SecureHttpClient};
use crate::paths::PathLayout;
use chrono::Utc;
use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use zip::ZipArchive;

pub const MINECRAFT_VERSION: &str = "1.8.9";
pub const FORGE_COORDINATE_VERSION: &str = "1.8.9-11.15.1.2318-1.8.9";
pub const FORGE_VERSION_ID: &str = "1.8.9-forge1.8.9-11.15.1.2318-1.8.9";
pub const FORGE_DISPLAY_VERSION: &str = "11.15.1.2318";

const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FORGE_INSTALLER_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/1.8.9-11.15.1.2318-1.8.9/forge-1.8.9-11.15.1.2318-1.8.9-installer.jar";
const FORGE_INSTALLER_SHA1: &str = "ec0293ff0776b8831f2ed90511bab76e635dda0c";
const FORGE_INSTALLER_SIZE: u64 = 4_485_999;
const FORGE_UNIVERSAL_ENTRY: &str = "forge-1.8.9-11.15.1.2318-1.8.9-universal.jar";
const ASSET_CONCURRENCY: usize = 8;
const EMBEDDED_PRIVATE_CLIENT_CORE_JAR: &[u8] = include_bytes!(
    "../../../../minecraft/private-client-core/build/libs/private-client-core-1.0.0.jar"
);
const EMBEDDED_SPLASH_BACKGROUND: &[u8] = include_bytes!(
    "../../../../minecraft/private-client-core/src/main/resources/assets/privateclientcore/textures/gui/loading-background.png"
);
const PRIVATE_CLIENT_SPLASH_PROPERTIES: &str = "\
enabled=true\n\
rotate=false\n\
logoOffset=-10000\n\
background=0x050505\n\
font=0xFFFFFF\n\
barBorder=0x454545\n\
bar=0xFFFFFF\n\
barBackground=0x171717\n\
fontTexture=textures/font/ascii.png\n\
logoTexture=privateclientcore:textures/gui/loading-background.png\n\
forgeTexture=privateclientcore:textures/gui/loading-background.png\n\
resourcePackPath=resources\n";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionMetadata {
    pub id: String,
    pub main_class: String,
    pub minecraft_arguments: String,
    pub assets: String,
    pub asset_index: AssetIndexRef,
    pub downloads: VersionDownloads,
    #[serde(default)]
    pub libraries: Vec<MojangLibrary>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadArtifact,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadArtifact {
    pub path: Option<String>,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangLibrary {
    pub name: String,
    #[serde(default)]
    pub downloads: LibraryDownloads,
    #[serde(default)]
    pub natives: BTreeMap<String, String>,
    #[serde(default)]
    pub rules: Vec<LibraryRule>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadArtifact>,
    #[serde(default)]
    pub classifiers: BTreeMap<String, DownloadArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryRule {
    pub action: String,
    pub os: Option<RuleOs>,
    pub features: Option<BTreeMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleOs {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionManifest {
    versions: Vec<VersionReference>,
}

#[derive(Debug, Deserialize)]
struct VersionReference {
    id: String,
    url: String,
    sha1: String,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    objects: BTreeMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForgeInstallerProfile {
    install: ForgeInstall,
    version_info: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForgeInstall {
    target: String,
    path: String,
    file_path: String,
    minecraft: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ForgeLibrary {
    name: String,
    url: Option<String>,
    checksums: Option<Vec<String>>,
    clientreq: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstanceStateFile {
    schema_version: u32,
    minecraft_version: String,
    forge_version: String,
    verified_at: String,
}

pub fn snapshot(paths: &PathLayout) -> InstanceSnapshot {
    let vanilla_directory = paths.versions.join(MINECRAFT_VERSION);
    let forge_directory = paths.versions.join(FORGE_VERSION_ID);
    let minecraft_ready = vanilla_directory
        .join(format!("{MINECRAFT_VERSION}.json"))
        .is_file()
        && vanilla_directory
            .join(format!("{MINECRAFT_VERSION}.jar"))
            .is_file();
    let forge_ready = forge_directory
        .join(format!("{FORGE_VERSION_ID}.json"))
        .is_file();
    let last_verified_at = if paths.instance_state.exists() {
        crate::fs_secure::read_json::<InstanceStateFile>(&paths.instance_state)
            .ok()
            .map(|state| state.verified_at)
    } else {
        None
    };
    InstanceSnapshot {
        installed: minecraft_ready && forge_ready,
        minecraft_ready,
        forge_ready,
        version: MINECRAFT_VERSION.to_owned(),
        forge_version: FORGE_DISPLAY_VERSION.to_owned(),
        last_verified_at,
    }
}

pub async fn install_or_repair(
    app: &AppHandle,
    paths: &PathLayout,
    network: &SecureHttpClient,
    logger: &LocalLogger,
    cancel: Arc<AtomicBool>,
) -> AppResult<InstanceSnapshot> {
    paths.ensure()?;
    ensure_not_cancelled(&cancel)?;
    emit_progress(
        app,
        "instance",
        "metadata",
        0.02,
        "Fetching Minecraft metadata",
    );
    let manifest: VersionManifest = network.get_json(VERSION_MANIFEST_URL).await?;
    let reference = manifest
        .versions
        .into_iter()
        .find(|version| version.id == MINECRAFT_VERSION)
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::MinecraftMetadataInvalid,
                "Official metadata does not contain Minecraft 1.8.9",
            )
        })?;
    ensure_not_cancelled(&cancel)?;
    let version_directory = paths.versions.join(MINECRAFT_VERSION);
    fs::create_dir_all(&version_directory)
        .map_err(|error| AppError::io("Could not create the Minecraft version directory", error))?;
    let metadata_path = version_directory.join(format!("{MINECRAFT_VERSION}.json"));
    let metadata_bytes = network.get_text(&reference.url, 8 * 1024 * 1024).await?;
    let metadata = parse_verified_version_metadata(&metadata_bytes, &reference.sha1)?;
    crate::fs_secure::atomic_write(&metadata_path, metadata_bytes.as_bytes())?;

    emit_progress(
        app,
        "instance",
        "client",
        0.08,
        "Verifying Minecraft client",
    );
    let client_path = version_directory.join(format!("{MINECRAFT_VERSION}.jar"));
    ensure_artifact(
        network,
        paths,
        &metadata.downloads.client,
        &client_path,
        64 * 1024 * 1024,
    )
    .await?;

    ensure_not_cancelled(&cancel)?;
    install_libraries(app, paths, network, &metadata, &cancel).await?;
    ensure_not_cancelled(&cancel)?;
    install_assets(app, paths, network, &metadata, &cancel).await?;
    ensure_not_cancelled(&cancel)?;
    install_forge(app, paths, network, logger).await?;
    ensure_not_cancelled(&cancel)?;
    install_bundled_modules(app, paths, logger)?;
    install_client_theme(paths)?;

    let state = InstanceStateFile {
        schema_version: 1,
        minecraft_version: MINECRAFT_VERSION.to_owned(),
        forge_version: FORGE_COORDINATE_VERSION.to_owned(),
        verified_at: Utc::now().to_rfc3339(),
    };
    atomic_write_json(&paths.instance_state, &state)?;
    emit_progress(app, "instance", "complete", 1.0, "Instance is ready");
    logger.info("instance", "Minecraft 1.8.9 Forge instance verified");
    Ok(snapshot(paths))
}

fn parse_verified_version_metadata(bytes: &str, expected_sha1: &str) -> AppResult<VersionMetadata> {
    let metadata_sha1 = sha1_hex(bytes.as_bytes());
    if !metadata_sha1.eq_ignore_ascii_case(expected_sha1) {
        return Err(AppError::new(
            AppErrorCode::HashMismatch,
            "Minecraft version metadata failed its provider hash",
        ));
    }
    let metadata: VersionMetadata = serde_json::from_str(bytes)
        .map_err(|error| AppError::json("Minecraft version metadata is invalid", error))?;
    validate_version_metadata(&metadata)?;
    Ok(metadata)
}

fn validate_version_metadata(metadata: &VersionMetadata) -> AppResult<()> {
    if metadata.id != MINECRAFT_VERSION
        || metadata.main_class != "net.minecraft.client.main.Main"
        || metadata.assets.is_empty()
        || metadata.minecraft_arguments.is_empty()
    {
        return Err(AppError::new(
            AppErrorCode::MinecraftMetadataInvalid,
            "Minecraft 1.8.9 metadata failed semantic validation",
        ));
    }
    Ok(())
}

async fn install_libraries(
    app: &AppHandle,
    paths: &PathLayout,
    network: &SecureHttpClient,
    metadata: &VersionMetadata,
    cancel: &Arc<AtomicBool>,
) -> AppResult<()> {
    crate::fs_secure::ensure_inside(&paths.instance, &paths.natives)?;
    if paths.natives.exists() {
        fs::remove_dir_all(&paths.natives)
            .map_err(|error| AppError::io("Could not reset verified native libraries", error))?;
    }
    fs::create_dir_all(&paths.natives)
        .map_err(|error| AppError::io("Could not create the native library directory", error))?;
    let applicable: Vec<&MojangLibrary> = metadata
        .libraries
        .iter()
        .filter(|library| library_applies(library))
        .collect();
    let total = applicable.len().max(1);
    for (index, library) in applicable.into_iter().enumerate() {
        ensure_not_cancelled(cancel)?;
        if let Some(artifact) = &library.downloads.artifact {
            let relative = artifact.path.as_deref().ok_or_else(|| {
                AppError::new(
                    AppErrorCode::MinecraftMetadataInvalid,
                    "A Minecraft library has no destination path",
                )
            })?;
            let destination = paths.libraries.join(safe_relative_path(relative)?);
            ensure_artifact(network, paths, artifact, &destination, JAR_LIMIT).await?;
        }
        if let Some(classifier_name) = library.natives.get("windows") {
            let classifier_name = classifier_name.replace("${arch}", "64");
            let artifact = library
                .downloads
                .classifiers
                .get(&classifier_name)
                .ok_or_else(|| {
                    AppError::new(
                        AppErrorCode::MinecraftMetadataInvalid,
                        "A Windows native library classifier is missing",
                    )
                    .details(library.name.clone())
                })?;
            let relative = artifact.path.as_deref().ok_or_else(|| {
                AppError::new(
                    AppErrorCode::MinecraftMetadataInvalid,
                    "A native library has no destination path",
                )
            })?;
            let destination = paths.libraries.join(safe_relative_path(relative)?);
            ensure_artifact(network, paths, artifact, &destination, JAR_LIMIT).await?;
            extract_zip_safely(&destination, &paths.natives)?;
        }
        let progress = 0.12 + (index as f32 / total as f32) * 0.23;
        emit_progress(
            app,
            "instance",
            "libraries",
            progress,
            "Verifying Minecraft libraries",
        );
    }
    Ok(())
}

async fn install_assets(
    app: &AppHandle,
    paths: &PathLayout,
    network: &SecureHttpClient,
    metadata: &VersionMetadata,
    cancel: &Arc<AtomicBool>,
) -> AppResult<()> {
    let index_path = paths
        .assets
        .join("indexes")
        .join(format!("{}.json", metadata.asset_index.id));
    let index_artifact = DownloadArtifact {
        path: None,
        url: metadata.asset_index.url.clone(),
        sha1: metadata.asset_index.sha1.clone(),
        size: metadata.asset_index.size,
    };
    ensure_artifact(
        network,
        paths,
        &index_artifact,
        &index_path,
        16 * 1024 * 1024,
    )
    .await?;
    let index: AssetIndex = crate::fs_secure::read_json(&index_path)?;
    let total = index.objects.len().max(1);
    let completed = Arc::new(AtomicUsize::new(0));
    let items: Vec<AssetObject> = index.objects.into_values().collect();
    let mut tasks = stream::iter(items)
        .map(|object| {
            let network = network.clone();
            let paths = paths.clone();
            let completed = Arc::clone(&completed);
            async move {
                validate_sha1_text(&object.hash)?;
                let prefix = object.hash.get(0..2).ok_or_else(|| {
                    AppError::new(
                        AppErrorCode::MinecraftMetadataInvalid,
                        "An asset hash is too short",
                    )
                })?;
                let destination = paths.assets.join("objects").join(prefix).join(&object.hash);
                let artifact = DownloadArtifact {
                    path: None,
                    url: format!(
                        "https://resources.download.minecraft.net/{prefix}/{}",
                        object.hash
                    ),
                    sha1: object.hash,
                    size: object.size,
                };
                ensure_artifact(&network, &paths, &artifact, &destination, 64 * 1024 * 1024)
                    .await?;
                completed.fetch_add(1, Ordering::Relaxed);
                AppResult::Ok(())
            }
        })
        .buffer_unordered(ASSET_CONCURRENCY);
    while let Some(result) = tasks.next().await {
        ensure_not_cancelled(cancel)?;
        result?;
        let count = completed.load(Ordering::Relaxed);
        if count % 25 == 0 || count == total {
            let progress = 0.38 + (count as f32 / total as f32) * 0.32;
            emit_progress(
                app,
                "instance",
                "assets",
                progress,
                "Verifying Minecraft assets",
            );
        }
    }
    Ok(())
}

async fn ensure_artifact(
    network: &SecureHttpClient,
    paths: &PathLayout,
    artifact: &DownloadArtifact,
    destination: &Path,
    maximum_size: u64,
) -> AppResult<()> {
    validate_sha1_text(&artifact.sha1)?;
    if destination.is_file() {
        let (_, actual_sha1, size) = hash_file(destination)?;
        if size == artifact.size && actual_sha1.eq_ignore_ascii_case(&artifact.sha1) {
            return Ok(());
        }
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| AppError::io("Could not create an artifact directory", error))?;
    }
    let temporary = paths.staging.join(format!("{}.part", Uuid::new_v4()));
    network
        .download(
            &artifact.url,
            &temporary,
            &DownloadExpectation {
                maximum_size,
                expected_size: Some(artifact.size),
                sha512: None,
                sha1: Some(artifact.sha1.clone()),
            },
        )
        .await?;
    replace_file(&temporary, destination)
}

async fn install_forge(
    app: &AppHandle,
    paths: &PathLayout,
    network: &SecureHttpClient,
    logger: &LocalLogger,
) -> AppResult<()> {
    emit_progress(
        app,
        "instance",
        "forge-installer",
        0.73,
        "Verifying official Forge installer",
    );
    let installer_directory = paths.downloads.join("forge");
    fs::create_dir_all(&installer_directory)
        .map_err(|error| AppError::io("Could not create the Forge cache", error))?;
    let installer = installer_directory.join("forge-1.8.9-11.15.1.2318-installer.jar");
    let needs_download = if installer.is_file() {
        let (_, sha1, size) = hash_file(&installer)?;
        size != FORGE_INSTALLER_SIZE || !sha1.eq_ignore_ascii_case(FORGE_INSTALLER_SHA1)
    } else {
        true
    };
    if needs_download {
        let temporary = paths.staging.join(format!("{}.part", Uuid::new_v4()));
        network
            .download(
                FORGE_INSTALLER_URL,
                &temporary,
                &DownloadExpectation {
                    maximum_size: 16 * 1024 * 1024,
                    expected_size: Some(FORGE_INSTALLER_SIZE),
                    sha512: None,
                    sha1: Some(FORGE_INSTALLER_SHA1.to_owned()),
                },
            )
            .await?;
        replace_file(&temporary, &installer)?;
    }
    validate_jar(&installer, 16 * 1024 * 1024)?;
    let profile = read_forge_profile(&installer)?;
    validate_forge_profile(&profile)?;
    install_forge_universal(&installer, &profile, paths)?;
    install_forge_libraries(app, paths, network, &profile).await?;

    let version_directory = paths.versions.join(&profile.install.target);
    fs::create_dir_all(&version_directory)
        .map_err(|error| AppError::io("Could not create the Forge version directory", error))?;
    atomic_write_json(
        &version_directory.join(format!("{}.json", profile.install.target)),
        &profile.version_info,
    )?;
    logger.info(
        "forge",
        "Installed Forge from the verified official installer profile",
    );
    Ok(())
}

fn read_forge_profile(installer: &Path) -> AppResult<ForgeInstallerProfile> {
    let file = File::open(installer)
        .map_err(|error| AppError::io("Could not open the Forge installer", error))?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry = archive.by_name("install_profile.json").map_err(|error| {
        AppError::new(
            AppErrorCode::ForgeInstallationFailed,
            "The Forge installer has no install profile",
        )
        .details(error.to_string())
    })?;
    if entry.size() > 2 * 1024 * 1024 {
        return Err(AppError::new(
            AppErrorCode::ForgeInstallationFailed,
            "The Forge install profile is unexpectedly large",
        ));
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| AppError::io("Could not read the Forge install profile", error))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| AppError::json("The Forge install profile is invalid", error))
}

fn validate_forge_profile(profile: &ForgeInstallerProfile) -> AppResult<()> {
    validate_identifier(&profile.install.target, "Forge target")?;
    if profile.install.target != FORGE_VERSION_ID
        || profile.install.minecraft != MINECRAFT_VERSION
        || profile.install.path != format!("net.minecraftforge:forge:{FORGE_COORDINATE_VERSION}")
        || profile.install.file_path != FORGE_UNIVERSAL_ENTRY
    {
        return Err(AppError::new(
            AppErrorCode::ForgeInstallationFailed,
            "The official Forge installer profile does not match the pinned release",
        ));
    }
    Ok(())
}

fn install_forge_universal(
    installer: &Path,
    profile: &ForgeInstallerProfile,
    paths: &PathLayout,
) -> AppResult<()> {
    let relative = maven_path(&profile.install.path)?;
    let destination = paths.libraries.join(relative);
    if fs::symlink_metadata(&destination).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(AppError::new(
            AppErrorCode::SymlinkDetected,
            "The Forge universal destination is a symbolic link",
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| AppError::io("Could not create the Forge library path", error))?;
    }
    let file = File::open(installer)
        .map_err(|error| AppError::io("Could not open the Forge installer", error))?;
    let mut archive = ZipArchive::new(file)?;
    let mut universal = archive
        .by_name(&profile.install.file_path)
        .map_err(|error| {
            AppError::new(
                AppErrorCode::ForgeInstallationFailed,
                "The Forge installer does not contain its universal JAR",
            )
            .details(error.to_string())
        })?;
    if universal.size() == 0 || universal.size() > 16 * 1024 * 1024 {
        return Err(AppError::new(
            AppErrorCode::ForgeInstallationFailed,
            "The embedded Forge universal JAR has an invalid size",
        ));
    }
    let temporary = paths.staging.join(format!("{}.jar", Uuid::new_v4()));
    {
        let mut output = File::create(&temporary)
            .map_err(|error| AppError::io("Could not stage the Forge universal JAR", error))?;
        std::io::copy(&mut universal, &mut output)
            .map_err(|error| AppError::io("Could not extract the Forge universal JAR", error))?;
        output
            .flush()
            .map_err(|error| AppError::io("Could not flush the Forge universal JAR", error))?;
    }
    validate_jar(&temporary, 16 * 1024 * 1024)?;
    let (expected_sha512, expected_sha1, expected_size) = hash_file(&temporary)?;
    let destination_matches = if destination.is_file() {
        validate_jar(&destination, 16 * 1024 * 1024).is_ok()
            && hash_file(&destination).is_ok_and(|(sha512, sha1, size)| {
                size == expected_size && sha512 == expected_sha512 && sha1 == expected_sha1
            })
    } else {
        false
    };
    if destination_matches {
        fs::remove_file(&temporary)
            .map_err(|error| AppError::io("Could not remove the Forge verification file", error))
    } else {
        replace_file(&temporary, &destination)
    }
}

async fn install_forge_libraries(
    app: &AppHandle,
    paths: &PathLayout,
    network: &SecureHttpClient,
    profile: &ForgeInstallerProfile,
) -> AppResult<()> {
    let libraries_value = profile
        .version_info
        .get("libraries")
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::ForgeInstallationFailed,
                "The Forge profile has no library list",
            )
        })?;
    let libraries: Vec<ForgeLibrary> = serde_json::from_value(libraries_value)
        .map_err(|error| AppError::json("Forge library metadata is invalid", error))?;
    let applicable: Vec<ForgeLibrary> = libraries
        .into_iter()
        .filter(|library| library.clientreq.unwrap_or(true))
        .filter(|library| library.name != profile.install.path)
        .collect();
    let total = applicable.len().max(1);
    for (index, library) in applicable.into_iter().enumerate() {
        let relative = maven_path(&library.name)?;
        let base = normalize_legacy_maven_base(library.url.as_deref())?;
        let url = format!(
            "{}/{}",
            base.trim_end_matches('/'),
            relative.to_string_lossy().replace('\\', "/")
        );
        let provider_hashes = if let Some(values) = library.checksums {
            values
                .into_iter()
                .filter(|value| is_sha1(value))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let provider_sha1 = if let Some(first) = provider_hashes.first() {
            first.clone()
        } else {
            fetch_sha1_sidecar(network, &format!("{url}.sha1")).await?
        };
        let destination = paths.libraries.join(&relative);
        if destination.is_file() {
            let (_, actual, _) = hash_file(&destination)?;
            let accepted = provider_hashes.is_empty()
                && actual.eq_ignore_ascii_case(&provider_sha1)
                || provider_hashes
                    .iter()
                    .any(|hash| actual.eq_ignore_ascii_case(hash));
            if accepted {
                continue;
            }
        }
        let temporary = forge_library_staging_path(paths);
        let receipt = network
            .download(
                &url,
                &temporary,
                &DownloadExpectation {
                    maximum_size: JAR_LIMIT,
                    expected_size: None,
                    sha512: None,
                    sha1: None,
                },
            )
            .await?;
        let accepted = provider_hashes.is_empty()
            && receipt.sha1.eq_ignore_ascii_case(&provider_sha1)
            || provider_hashes
                .iter()
                .any(|hash| receipt.sha1.eq_ignore_ascii_case(hash));
        if !accepted {
            let _ = fs::remove_file(&temporary);
            return Err(AppError::new(
                AppErrorCode::HashMismatch,
                "A Forge library failed its provider checksum",
            )
            .details(library.name));
        }
        validate_jar(&temporary, JAR_LIMIT)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| AppError::io("Could not create a Forge library path", error))?;
        }
        replace_file(&temporary, &destination)?;
        emit_progress(
            app,
            "instance",
            "forge-libraries",
            0.76 + (index as f32 / total as f32) * 0.18,
            "Installing Forge libraries",
        );
    }
    Ok(())
}

fn forge_library_staging_path(paths: &PathLayout) -> PathBuf {
    paths.staging.join(format!("{}.jar", Uuid::new_v4()))
}

fn normalize_legacy_maven_base(value: Option<&str>) -> AppResult<String> {
    let Some(value) = value else {
        return Ok("https://libraries.minecraft.net/".to_owned());
    };
    let trimmed = value.trim_end_matches('/');
    if matches!(
        trimmed,
        "http://files.minecraftforge.net/maven"
            | "https://files.minecraftforge.net/maven"
            | "http://maven.minecraftforge.net"
            | "https://maven.minecraftforge.net"
    ) {
        return Ok("https://maven.minecraftforge.net/".to_owned());
    }
    if value.starts_with("http://") {
        return Err(AppError::new(
            AppErrorCode::UntrustedHost,
            "A legacy Forge profile attempted to use an unapproved HTTP repository",
        )
        .details(value.to_owned()));
    }
    crate::network::validate_url_text(value)?;
    Ok(format!("{trimmed}/"))
}

async fn fetch_sha1_sidecar(network: &SecureHttpClient, url: &str) -> AppResult<String> {
    let text = network.get_text(url, 16 * 1024).await?;
    let hash = text
        .split_whitespace()
        .find(|value| is_sha1(value))
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::ManifestInvalid,
                "A Maven repository returned an invalid SHA-1 sidecar",
            )
        })?;
    Ok(hash.to_ascii_lowercase())
}

fn install_bundled_modules(
    _app: &AppHandle,
    paths: &PathLayout,
    _logger: &LocalLogger,
) -> AppResult<()> {
    let legacy_files = [
        "toggle-sprint-1.0.0.jar",
        "private-nametags-1.0.0.jar",
        "private-freelook-1.0.0.jar",
        "private-optimization-1.0.0.jar",
        "hit-delay-fix-1.0.0.jar",
        "item-physics-1.0.0.jar",
        "scoreboard-mod-1.0.0.jar",
    ];
    for file_name in legacy_files {
        let legacy_path = paths.mods.join(file_name);
        if legacy_path.is_file() {
            let _ = std::fs::remove_file(&legacy_path);
        }
    }

    let modules = [(
        "private-client-core-1.0.0.jar",
        EMBEDDED_PRIVATE_CLIENT_CORE_JAR,
    )];
    for (file_name, bytes) in modules {
        if bytes.is_empty() {
            return Err(AppError::new(
                AppErrorCode::InstanceCorrupted,
                "An embedded module JAR is empty",
            ));
        }
        let destination = paths.mods.join(file_name);
        let (source_hash, _) = hash_bytes(bytes);
        let needs_copy = if destination.is_file() {
            hash_file(&destination)?.0 != source_hash
        } else {
            true
        };
        if needs_copy {
            atomic_write(&destination, bytes)?;
        }
        validate_jar(&destination, 64 * 1024 * 1024)?;
    }
    Ok(())
}

fn install_client_theme(paths: &PathLayout) -> AppResult<()> {
    let splash_config = paths.instance.join("config").join("splash.properties");
    let splash_background = paths
        .instance
        .join("resources")
        .join("assets")
        .join("privateclientcore")
        .join("textures")
        .join("gui")
        .join("loading-background.png");
    crate::fs_secure::ensure_inside(&paths.instance, &splash_config)?;
    crate::fs_secure::ensure_inside(&paths.instance, &splash_background)?;
    atomic_write(&splash_config, PRIVATE_CLIENT_SPLASH_PROPERTIES.as_bytes())?;
    atomic_write(&splash_background, EMBEDDED_SPLASH_BACKGROUND)
}

pub(crate) fn embedded_private_client_core_jar() -> &'static [u8] {
    EMBEDDED_PRIVATE_CLIENT_CORE_JAR
}

pub fn library_applies(library: &MojangLibrary) -> bool {
    if library.rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in &library.rules {
        if rule_matches(rule) {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

fn rule_matches(rule: &LibraryRule) -> bool {
    if rule
        .features
        .as_ref()
        .is_some_and(|features| features.values().any(|required| *required))
    {
        return false;
    }
    let Some(os) = &rule.os else {
        return true;
    };
    if os
        .name
        .as_deref()
        .is_some_and(|name| !name.eq_ignore_ascii_case("windows"))
    {
        return false;
    }
    if os.arch.as_deref().is_some_and(|arch| {
        !matches!(
            arch.to_ascii_lowercase().as_str(),
            "x86_64" | "amd64" | "x64"
        )
    }) {
        return false;
    }
    let _ = &os.version;
    true
}

pub fn maven_path(coordinate: &str) -> AppResult<PathBuf> {
    let segments: Vec<&str> = coordinate.split(':').collect();
    if !(3..=4).contains(&segments.len()) {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "A Maven coordinate is invalid",
        )
        .details(coordinate.to_owned()));
    }
    for segment in &segments {
        validate_identifier(segment, "Maven coordinate")?;
    }
    let group_path = segments[0].replace('.', "/");
    let artifact = segments[1];
    let version = segments[2];
    let classifier = segments
        .get(3)
        .map(|value| format!("-{value}"))
        .unwrap_or_default();
    safe_relative_path(&format!(
        "{group_path}/{artifact}/{version}/{artifact}-{version}{classifier}.jar"
    ))
}

fn validate_sha1_text(value: &str) -> AppResult<()> {
    if is_sha1(value) {
        Ok(())
    } else {
        Err(AppError::new(
            AppErrorCode::MinecraftMetadataInvalid,
            "Provider metadata contains an invalid SHA-1 hash",
        ))
    }
}

fn is_sha1(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut digest = Sha1::new();
    digest.update(bytes);
    hex::encode(digest.finalize())
}

fn ensure_not_cancelled(cancel: &AtomicBool) -> AppResult<()> {
    if cancel.load(Ordering::SeqCst) {
        return Err(AppError::new(
            AppErrorCode::OperationBlockedWhileRunning,
            "The launch preparation was cancelled",
        ));
    }
    Ok(())
}

fn emit_progress(app: &AppHandle, domain: &str, stage: &str, progress: f32, message: &str) {
    let event = ProgressEvent {
        operation_id: "instance-preparation".to_owned(),
        target_id: domain.to_owned(),
        phase: stage.to_owned(),
        progress: progress.clamp(0.0, 1.0) * 100.0,
        message: message.to_owned(),
    };
    let _ = app.emit(EVENT_OPERATION_PROGRESS, event);
}

#[cfg(test)]
mod tests {
    use super::{
        forge_library_staging_path, install_client_theme, library_applies, maven_path,
        normalize_legacy_maven_base, parse_verified_version_metadata, sha1_hex, ForgeInstall,
        ForgeInstallerProfile, LibraryDownloads, LibraryRule, MojangLibrary, RuleOs,
        FORGE_COORDINATE_VERSION, FORGE_UNIVERSAL_ENTRY, FORGE_VERSION_ID,
    };
    use crate::fs_secure::{atomic_write, hash_file};
    use crate::paths::PathLayout;
    use std::collections::BTreeMap;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn library(rules: Vec<LibraryRule>) -> MojangLibrary {
        MojangLibrary {
            name: "group:artifact:1.0".to_owned(),
            downloads: LibraryDownloads::default(),
            natives: BTreeMap::new(),
            rules,
        }
    }

    #[test]
    fn converts_maven_coordinates_safely() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            maven_path("net.minecraft:launchwrapper:1.12")?,
            std::path::PathBuf::from("net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar")
        );
        assert!(maven_path("../../evil:artifact:1").is_err());
        Ok(())
    }

    #[test]
    fn evaluates_windows_rules() {
        let allowed = library(vec![LibraryRule {
            action: "allow".to_owned(),
            os: Some(RuleOs {
                name: Some("windows".to_owned()),
                arch: None,
                version: None,
            }),
            features: None,
        }]);
        assert!(library_applies(&allowed));
        let linux = library(vec![LibraryRule {
            action: "allow".to_owned(),
            os: Some(RuleOs {
                name: Some("linux".to_owned()),
                arch: None,
                version: None,
            }),
            features: None,
        }]);
        assert!(!library_applies(&linux));
    }

    #[test]
    fn upgrades_only_known_legacy_forge_maven_hosts() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            normalize_legacy_maven_base(Some("http://files.minecraftforge.net/maven"))?,
            "https://maven.minecraftforge.net/"
        );
        assert!(normalize_legacy_maven_base(Some("http://example.com/maven")).is_err());
        Ok(())
    }

    #[test]
    fn stages_downloaded_forge_libraries_with_a_jar_extension(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let paths = PathLayout::for_root(temporary.path().join("Private Client"))?;
        let staged = forge_library_staging_path(&paths);
        assert_eq!(
            staged.extension().and_then(std::ffi::OsStr::to_str),
            Some("jar")
        );
        Ok(())
    }

    #[test]
    fn module_jars_are_embedded_for_standalone_launches() {
        let bytes = super::embedded_private_client_core_jar();
        assert!(bytes.len() > 4);
        assert_eq!(&bytes[..4], b"PK\x03\x04");
    }

    #[test]
    fn provisions_the_fixed_monochrome_forge_splash() -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let paths = PathLayout::for_root(temporary.path().join("Private Client"))?;
        paths.ensure()?;
        install_client_theme(&paths)?;

        let properties =
            std::fs::read_to_string(paths.instance.join("config").join("splash.properties"))?;
        assert!(properties.contains("background=0x050505"));
        assert!(properties
            .contains("logoTexture=privateclientcore:textures/gui/loading-background.png",));
        let image = std::fs::read(
            paths
                .instance
                .join("resources/assets/privateclientcore/textures/gui/loading-background.png"),
        )?;
        assert_eq!(&image[..8], b"\x89PNG\r\n\x1a\n");

        // Forge stretches this across the whole splash viewport, so the source
        // has to be at least full HD and already in the display aspect. A
        // low-resolution or square source is visibly soft once stretched, which
        // is the regression worth catching here.
        let width = u32::from_be_bytes(image[16..20].try_into()?);
        let height = u32::from_be_bytes(image[20..24].try_into()?);
        assert!(width >= 1920, "splash background is {width}px wide");
        assert!(height >= 1080, "splash background is {height}px tall");
        assert!(
            (width as f64 / height as f64 - 16.0 / 9.0).abs() < 0.01,
            "splash background should be 16:9, got {width}x{height}"
        );
        Ok(())
    }

    #[test]
    fn hashes_version_metadata_before_parsing_or_use() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"id":"1.8.9","mainClass":"net.minecraft.client.main.Main","minecraftArguments":"--username ${auth_player_name}","assets":"legacy","assetIndex":{"id":"1.8","url":"https://piston-meta.mojang.com/a","sha1":"0000000000000000000000000000000000000000","size":1},"downloads":{"client":{"path":null,"url":"https://piston-data.mojang.com/a","sha1":"0000000000000000000000000000000000000000","size":1}},"libraries":[]}"#;
        let hash = sha1_hex(json.as_bytes());
        assert!(parse_verified_version_metadata(json, &hash).is_ok());
        assert!(
            parse_verified_version_metadata(json, "1111111111111111111111111111111111111111")
                .is_err()
        );
        Ok(())
    }

    fn test_mod_jar(marker: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())?;
        archive.write_all(format!("Manifest-Version: 1.0\n{marker}\n").as_bytes())?;
        Ok(archive.finish()?.into_inner())
    }

    #[test]
    fn repairs_a_tampered_forge_universal_from_the_pinned_installer(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let paths = PathLayout::for_root(temporary.path().join("Private Client"))?;
        paths.ensure()?;
        let installer = temporary.path().join("forge-installer.jar");
        let expected_jar = test_mod_jar("expected")?;
        let mut archive = ZipWriter::new(std::fs::File::create(&installer)?);
        archive.start_file(FORGE_UNIVERSAL_ENTRY, SimpleFileOptions::default())?;
        archive.write_all(&expected_jar)?;
        archive.finish()?;
        let profile = ForgeInstallerProfile {
            install: ForgeInstall {
                target: FORGE_VERSION_ID.to_owned(),
                path: format!("net.minecraftforge:forge:{FORGE_COORDINATE_VERSION}"),
                file_path: FORGE_UNIVERSAL_ENTRY.to_owned(),
                minecraft: "1.8.9".to_owned(),
            },
            version_info: serde_json::json!({}),
        };
        super::install_forge_universal(&installer, &profile, &paths)?;
        let destination = paths.libraries.join(maven_path(&profile.install.path)?);
        let expected_hash = hash_file(&destination)?.0;
        atomic_write(&destination, &test_mod_jar("tampered")?)?;
        assert_ne!(hash_file(&destination)?.0, expected_hash);
        super::install_forge_universal(&installer, &profile, &paths)?;
        assert_eq!(hash_file(&destination)?.0, expected_hash);
        Ok(())
    }
}
