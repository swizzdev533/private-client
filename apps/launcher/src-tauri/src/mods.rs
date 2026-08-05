use crate::contracts::{
    InstallModRequest, InstalledMod, ModCompatibility, ModEnvironment, ModInstallPlan,
    ModOperationResult, ModSource, ModTrust, ModsChangedEvent, ProgressEvent, QueueOperationKind,
    QueueOperationStatus, QueuedOperation, ReleaseType, RemoveModRequest, UpdateModRequest,
    EVENT_MODS_CHANGED, EVENT_OPERATION_PROGRESS,
};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::{
    atomic_copy, atomic_write, atomic_write_json, ensure_inside, hash_bytes, hash_file, read_json,
    safe_relative_path, validate_identifier, validate_jar, JAR_LIMIT,
};
use crate::modrinth::{self, ResolvedInstallPlan};
use crate::network::{validate_url_text, DownloadExpectation};
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Emitter;
use uuid::Uuid;

const REQUIRED_PRIVATE_CLIENT_CORE_ID: &str = "private-client-core";
const REQUIRED_PRIVATE_CLIENT_CORE_FILE: &str = "private-client-core-1.0.0.jar";
const REQUIRED_IAS_ID: &str = "in-game-account-switcher";
const REQUIRED_IAS_PROJECT_ID: &str = "cudtvDnd";
const REQUIRED_IAS_VERSION_ID: &str = "uI9n4nDb";
const REQUIRED_IAS_URL: &str = "https://cdn.modrinth.com//data/cudtvDnd/versions/7.1.2-fo1.8.9/InGameAccountSwitcher-Forge-1.8.9-7.1.2.jar";
const REQUIRED_IAS_SHA512: &str = "a6c9b1092fc30f7eb7946b4c865039506f9cc38331d628140a0a390969779b236586580bb5f9e30875cf4e43fce2751906fe0f0df333344625c707b93d1ef5ba";
const REQUIRED_IAS_SHA1: &str = "be37574260a921d0e9e9518890babf9cc2fd9230";
const EMBEDDED_REQUIRED_MODS_JSON: &str =
    include_str!("../../../../manifests/mods/required-mods.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledDatabase {
    schema_version: u32,
    mods: Vec<InstalledMod>,
}

impl Default for InstalledDatabase {
    fn default() -> Self {
        Self {
            schema_version: 1,
            mods: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationQueue {
    schema_version: u32,
    operations: Vec<QueuedOperation>,
}

impl Default for OperationQueue {
    fn default() -> Self {
        Self {
            schema_version: 1,
            operations: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RequiredModsManifest {
    schema_version: u32,
    minecraft_version: String,
    loader: String,
    #[serde(default)]
    mods: Vec<RequiredModEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequiredModEntry {
    id: String,
    name: String,
    provider: String,
    version: String,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    version_id: Option<String>,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    sha512: Option<String>,
    #[serde(default)]
    sha1: Option<String>,
    required: bool,
    removable: bool,
    license: String,
    #[serde(default)]
    notes: Option<String>,
}

struct StagedMod {
    path: PathBuf,
    record: InstalledMod,
}

struct RequiredRecordInput<'a> {
    project_id: &'a str,
    version_id: &'a str,
    file_name: &'a str,
    sha512: String,
    file_size: u64,
    provider: ModSource,
    trust: ModTrust,
    author: &'a str,
    description: &'a str,
}

pub fn list_installed(state: &AppState) -> AppResult<Vec<InstalledMod>> {
    let mut database = load_database(state)?;
    append_build_records(state, &mut database.mods)?;
    append_required_placeholders(&mut database.mods);
    database.mods.retain(|installed| {
        installed.project_id != "private-client-core"
            && installed.id != "private-client-core"
            && installed.project_id != "toggle-sprint"
            && installed.project_id != "private-nametags"
            && installed.project_id != "private-freelook"
            && installed.project_id != "private-optimization"
            && installed.project_id != "hit-delay-fix"
            && installed.project_id != "item-physics"
            && installed.project_id != "scoreboard-mod"
            && installed.id != "toggle-sprint"
            && installed.id != "private-nametags"
            && installed.id != "private-freelook"
            && installed.id != "private-optimization"
            && installed.id != "hit-delay-fix"
            && installed.id != "item-physics"
            && installed.id != "scoreboard-mod"
    });
    database
        .mods
        .sort_by_key(|installed| installed.name.to_lowercase());
    Ok(database.mods)
}

fn append_required_placeholders(installed_mods: &mut Vec<InstalledMod>) {
    if let Ok(manifest) = load_required_manifest() {
        for entry in manifest.mods.iter().filter(|e| e.required) {
            if entry.provider == "private-client-build" {
                continue;
            }
            let pid = entry.project_id.as_deref().unwrap_or(&entry.id);
            if !installed_mods
                .iter()
                .any(|m| m.id == entry.id || m.project_id == pid)
            {
                installed_mods.push(placeholder_required_record(entry));
            }
        }
    }
}

fn placeholder_required_record(entry: &RequiredModEntry) -> InstalledMod {
    let pid = entry.project_id.as_deref().unwrap_or(&entry.id);
    let vid = entry.version_id.as_deref().unwrap_or(&entry.version);
    let fname = entry.file_name.as_deref().unwrap_or("mod.jar");
    let desc = entry
        .notes
        .clone()
        .unwrap_or_else(|| "Pinned required mod for 1.8.9 Forge optimization.".to_owned());
    InstalledMod {
        id: entry.id.clone(),
        project_id: pid.to_owned(),
        version_id: vid.to_owned(),
        name: entry.name.clone(),
        author: "Modrinth project".to_owned(),
        description: desc,
        icon_url: None,
        version: entry.version.clone(),
        release_type: ReleaseType::Release,
        downloads: 0,
        updated_at: Utc::now().to_rfc3339(),
        minecraft_version: crate::minecraft::MINECRAFT_VERSION.to_owned(),
        loader: "forge".to_owned(),
        environment: ModEnvironment::Client,
        license: entry.license.clone(),
        file_size: 0,
        dependency_count: 0,
        trust: ModTrust::FromModrinth,
        compatibility: ModCompatibility::Compatible,
        compatibility_reason: None,
        installed: true,
        installed_version: Some(entry.version.clone()),
        update_available: false,
        required: true,
        file_name: fname.to_owned(),
        sha512: entry.sha512.clone().unwrap_or_default(),
        installed_at: Utc::now().to_rfc3339(),
        dependencies: Vec::new(),
        dependents: Vec::new(),
        provider: ModSource::Modrinth,
    }
}

pub fn list_pending(state: &AppState) -> AppResult<Vec<QueuedOperation>> {
    Ok(load_queue(state)?.operations)
}

pub async fn ensure_required_mods(state: &AppState) -> AppResult<()> {
    let manifest = load_required_manifest()?;
    let mut database = load_database(state)?;
    let mut ias_seen = false;

    let valid_required_ids: BTreeSet<String> = manifest
        .mods
        .iter()
        .filter(|entry| entry.required)
        .map(|entry| entry.id.clone())
        .collect();
    let valid_required_pids: BTreeSet<String> = manifest
        .mods
        .iter()
        .filter(|entry| entry.required)
        .map(|entry| entry.project_id.clone().unwrap_or_else(|| entry.id.clone()))
        .collect();

    let obsolete_mods: Vec<InstalledMod> = database
        .mods
        .iter()
        .filter(|installed| {
            installed.required
                && !valid_required_ids.contains(&installed.id)
                && !valid_required_pids.contains(&installed.project_id)
        })
        .cloned()
        .collect();

    for obsolete in obsolete_mods {
        if !obsolete.file_name.is_empty() {
            if let Ok(safe_name) = safe_relative_path(&obsolete.file_name) {
                let target = state.paths.mods.join(safe_name);
                if target.is_file() {
                    let _ = fs::remove_file(&target);
                }
            }
        }
        database.mods.retain(|installed| {
            installed.id != obsolete.id && installed.project_id != obsolete.project_id
        });
    }

    for entry in manifest.mods.iter() {
        if entry.required && entry.removable {
            return Err(AppError::new(
                AppErrorCode::ManifestInvalid,
                "A required mod cannot be marked as removable",
            )
            .details(entry.id.as_str()));
        }
        let record = match entry.provider.as_str() {
            "private-client-build" => ensure_required_internal_module(state, entry)?,
            "modrinth" => {
                if entry.id == REQUIRED_IAS_ID {
                    validate_pinned_ias_entry(entry)?;
                    ias_seen = true;
                }
                ensure_required_modrinth_file(state, entry).await?
            }
            _ => {
                return Err(AppError::new(
                    AppErrorCode::ManifestInvalid,
                    "The required-mod manifest contains an unsupported provider",
                )
                .details(entry.provider.as_str()));
            }
        };
        database
            .mods
            .retain(|installed| installed.project_id != record.project_id);
        database.mods.push(record);
    }
    if !ias_seen {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "The required-mod manifest must pin IAS",
        ));
    }
    save_database(state, &database)?;
    emit_mods_changed(state)?;
    Ok(())
}

fn ensure_required_internal_module(
    state: &AppState,
    entry: &RequiredModEntry,
) -> AppResult<InstalledMod> {
    let file_name = entry.file_name.as_deref().ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "The internal module manifest entry has no file name",
        )
    })?;

    let (source, destination_path) = match entry.id.as_str() {
        REQUIRED_PRIVATE_CLIENT_CORE_ID => (
            crate::minecraft::embedded_private_client_core_jar(),
            state
                .paths
                .mods
                .join(safe_relative_path(REQUIRED_PRIVATE_CLIENT_CORE_FILE)?),
        ),
        _ => {
            return Err(
                AppError::new(AppErrorCode::ManifestInvalid, "Unknown internal module ID")
                    .details(entry.id.as_str()),
            );
        }
    };

    ensure_regular_or_missing(&destination_path)?;
    let (expected_sha512, expected_sha1) = hash_bytes(source);
    let needs_repair = if destination_path.is_file() {
        let (sha512, sha1, _) = hash_file(&destination_path)?;
        sha512 != expected_sha512 || sha1 != expected_sha1
    } else {
        true
    };
    if needs_repair {
        atomic_write(&destination_path, source)?;
    }
    validate_jar(&destination_path, JAR_LIMIT)?;
    let (sha512, _, size) = hash_file(&destination_path)?;
    let version_id = format!("{}-{}", entry.id, entry.version);
    Ok(required_record(
        entry,
        RequiredRecordInput {
            project_id: &entry.id,
            version_id: &version_id,
            file_name,
            sha512,
            file_size: size,
            provider: ModSource::PrivateClient,
            trust: ModTrust::Verified,
            author: "Private Client",
            description: entry
                .notes
                .as_deref()
                .unwrap_or("Required Private Client module."),
        },
    ))
}

async fn ensure_required_modrinth_file(
    state: &AppState,
    entry: &RequiredModEntry,
) -> AppResult<InstalledMod> {
    let project_id = entry.project_id.as_deref().ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "The required Modrinth entry has no project ID",
        )
    })?;
    let version_id = entry.version_id.as_deref().ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "The required Modrinth entry has no version ID",
        )
    })?;
    let url = entry.url.as_deref().ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "The required Modrinth entry has no download URL",
        )
    })?;
    let sha512 = entry.sha512.as_deref().ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "The required Modrinth entry has no SHA-512",
        )
    })?;
    let sha1 = entry.sha1.as_deref().ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "The required Modrinth entry has no SHA-1",
        )
    })?;
    validate_identifier(project_id, "Required Modrinth project ID")?;
    validate_identifier(version_id, "Required Modrinth version ID")?;
    validate_hash(sha512, 128, "SHA-512")?;
    validate_hash(sha1, 40, "SHA-1")?;
    let parsed = validate_url_text(url)?;
    let file_name = parsed
        .path_segments()
        .and_then(Iterator::last)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::ManifestInvalid,
                "The required mod URL has no file name",
            )
        })?;
    let _ = safe_relative_path(file_name)?;
    let destination = state.paths.mods.join(file_name);
    ensure_regular_or_missing(&destination)?;
    let existing_valid = if destination.is_file() {
        let (actual_sha512, actual_sha1, _) = hash_file(&destination)?;
        actual_sha512.eq_ignore_ascii_case(sha512)
            && actual_sha1.eq_ignore_ascii_case(sha1)
            && validate_jar(&destination, JAR_LIMIT).is_ok()
    } else {
        false
    };
    if !existing_valid {
        let staging = state
            .paths
            .staging
            .join(format!("required-mod-{}.jar", Uuid::new_v4()));
        ensure_inside(&state.paths.staging, &staging)?;
        let result = async {
            state
                .network
                .download(
                    url,
                    &staging,
                    &DownloadExpectation {
                        maximum_size: JAR_LIMIT,
                        expected_size: None,
                        sha512: Some(sha512.to_owned()),
                        sha1: Some(sha1.to_owned()),
                    },
                )
                .await?;
            validate_jar(&staging, JAR_LIMIT)?;
            atomic_copy(&staging, &destination)
        }
        .await;
        let _ = fs::remove_file(&staging);
        result?;
    }
    validate_jar(&destination, JAR_LIMIT)?;
    let (actual_sha512, actual_sha1, size) = hash_file(&destination)?;
    if !actual_sha512.eq_ignore_ascii_case(sha512) || !actual_sha1.eq_ignore_ascii_case(sha1) {
        return Err(AppError::new(
            AppErrorCode::HashMismatch,
            "The required IAS JAR failed pinned hash verification",
        ));
    }
    let desc = entry
        .notes
        .as_deref()
        .unwrap_or("Pinned required Modrinth mod for Forge 1.8.9.");
    Ok(required_record(
        entry,
        RequiredRecordInput {
            project_id,
            version_id,
            file_name,
            sha512: actual_sha512,
            file_size: size,
            provider: ModSource::Modrinth,
            trust: ModTrust::FromModrinth,
            author: "Modrinth project",
            description: desc,
        },
    ))
}

fn required_record(entry: &RequiredModEntry, input: RequiredRecordInput<'_>) -> InstalledMod {
    InstalledMod {
        id: entry.id.clone(),
        project_id: input.project_id.to_owned(),
        version_id: input.version_id.to_owned(),
        name: entry.name.clone(),
        author: input.author.to_owned(),
        description: input.description.to_owned(),
        icon_url: None,
        version: entry.version.clone(),
        release_type: ReleaseType::Release,
        downloads: 0,
        updated_at: Utc::now().to_rfc3339(),
        minecraft_version: crate::minecraft::MINECRAFT_VERSION.to_owned(),
        loader: "forge".to_owned(),
        environment: ModEnvironment::Client,
        license: entry.license.clone(),
        file_size: input.file_size,
        dependency_count: 0,
        trust: input.trust,
        compatibility: ModCompatibility::Compatible,
        compatibility_reason: None,
        installed: true,
        installed_version: Some(entry.version.clone()),
        update_available: false,
        required: true,
        file_name: input.file_name.to_owned(),
        sha512: input.sha512,
        installed_at: Utc::now().to_rfc3339(),
        dependencies: Vec::new(),
        dependents: Vec::new(),
        provider: input.provider,
    }
}

fn ensure_regular_or_missing(path: &Path) -> AppResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(AppError::new(
            AppErrorCode::SymlinkDetected,
            "A required mod destination is a symbolic link",
        )),
        Ok(metadata) if !metadata.is_file() => Err(AppError::new(
            AppErrorCode::JarValidationFailed,
            "A required mod destination is not a regular file",
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::io(
            "Could not inspect a required mod destination",
            error,
        )),
    }
}

fn validate_pinned_ias_entry(entry: &RequiredModEntry) -> AppResult<()> {
    let pinned = entry.id == REQUIRED_IAS_ID
        && entry.project_id.as_deref() == Some(REQUIRED_IAS_PROJECT_ID)
        && entry.version_id.as_deref() == Some(REQUIRED_IAS_VERSION_ID)
        && entry.url.as_deref() == Some(REQUIRED_IAS_URL)
        && entry.sha512.as_deref() == Some(REQUIRED_IAS_SHA512)
        && entry.sha1.as_deref() == Some(REQUIRED_IAS_SHA1);
    if pinned {
        Ok(())
    } else {
        Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "The IAS required-mod entry does not match the compiled release pin",
        ))
    }
}

fn validate_hash(value: &str, length: usize, label: &str) -> AppResult<()> {
    if value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            format!("The required mod {label} is invalid"),
        ))
    }
}

pub async fn get_install_plan(
    state: &AppState,
    request: &InstallModRequest,
) -> AppResult<ModInstallPlan> {
    Ok(modrinth::resolve_plan(
        &state.network,
        &request.project_id,
        request.version_id.as_deref(),
        request.allow_beta,
    )
    .await?
    .public)
}

pub async fn install(
    state: &AppState,
    request: InstallModRequest,
) -> AppResult<ModOperationResult> {
    validate_identifier(&request.project_id, "Modrinth project ID")?;
    reject_required_mod_mutation(state, &request.project_id)?;
    let _guard = state.operation_lock.lock().await;
    if state.is_game_running() {
        let operation = QueuedOperation {
            id: Uuid::new_v4().to_string(),
            operation_type: QueueOperationKind::Install,
            target_id: request.project_id.clone(),
            target_name: request.project_id,
            created_at: Utc::now().to_rfc3339(),
            status: QueueOperationStatus::Pending,
            retry_count: 0,
            error_message: None,
            version_id: request.version_id,
            local_path: None,
            allow_beta: request.allow_beta,
        };
        enqueue(state, operation.clone())?;
        return Ok(ModOperationResult {
            operation_id: operation.id,
            queued: true,
            installed: Vec::new(),
        });
    }
    install_now(state, request).await
}

pub async fn update(state: &AppState, request: UpdateModRequest) -> AppResult<ModOperationResult> {
    validate_identifier(&request.project_id, "Modrinth project ID")?;
    reject_required_mod_mutation(state, &request.project_id)?;
    let _guard = state.operation_lock.lock().await;
    if state.is_game_running() {
        let operation = QueuedOperation {
            id: Uuid::new_v4().to_string(),
            operation_type: QueueOperationKind::Update,
            target_id: request.project_id.clone(),
            target_name: request.project_id,
            created_at: Utc::now().to_rfc3339(),
            status: QueueOperationStatus::Pending,
            retry_count: 0,
            error_message: None,
            version_id: None,
            local_path: None,
            allow_beta: request.allow_beta,
        };
        enqueue(state, operation.clone())?;
        return Ok(ModOperationResult {
            operation_id: operation.id,
            queued: true,
            installed: Vec::new(),
        });
    }
    ensure_installed(state, &request.project_id)?;
    install_now(
        state,
        InstallModRequest {
            project_id: request.project_id,
            version_id: None,
            allow_beta: request.allow_beta,
        },
    )
    .await
}

pub async fn remove(state: &AppState, request: RemoveModRequest) -> AppResult<ModOperationResult> {
    validate_identifier(&request.project_id, "Mod project ID")?;
    reject_required_mod_mutation(state, &request.project_id)?;
    let _guard = state.operation_lock.lock().await;
    if state.is_game_running() {
        let operation = QueuedOperation {
            id: Uuid::new_v4().to_string(),
            operation_type: QueueOperationKind::Remove,
            target_id: request.project_id.clone(),
            target_name: request.project_id,
            created_at: Utc::now().to_rfc3339(),
            status: QueueOperationStatus::Pending,
            retry_count: 0,
            error_message: None,
            version_id: None,
            local_path: None,
            allow_beta: false,
        };
        enqueue(state, operation.clone())?;
        return Ok(ModOperationResult {
            operation_id: operation.id,
            queued: true,
            installed: Vec::new(),
        });
    }
    remove_now(state, &request.project_id)
}

pub async fn cancel_pending(
    state: &AppState,
    operation_id: &str,
) -> AppResult<Vec<QueuedOperation>> {
    validate_uuid(operation_id)?;
    let _guard = state.operation_lock.lock().await;
    let mut queue = load_queue(state)?;
    let initial = queue.operations.len();
    queue
        .operations
        .retain(|operation| operation.id != operation_id);
    if queue.operations.len() == initial {
        return Err(AppError::new(
            AppErrorCode::ModNotFound,
            "The queued operation was not found",
        ));
    }
    save_queue(state, &queue)?;
    Ok(queue.operations)
}

pub async fn apply_pending(state: &AppState) -> AppResult<Vec<QueuedOperation>> {
    let _guard = state.operation_lock.lock().await;
    if state.is_game_running() {
        return Err(AppError::new(
            AppErrorCode::OperationBlockedWhileRunning,
            "Pending mod operations cannot run while the game is active",
        ));
    }
    let mut queue = load_queue(state)?;
    let operations = queue.operations.clone();
    for operation in operations {
        if !matches!(operation.operation_type, QueueOperationKind::ImportOptifine) {
            reject_required_mod_mutation(state, &operation.target_id)?;
        }
        let result = match operation.operation_type {
            QueueOperationKind::Install | QueueOperationKind::Update => install_now(
                state,
                InstallModRequest {
                    project_id: operation.target_id.clone(),
                    version_id: operation.version_id.clone(),
                    allow_beta: operation.allow_beta,
                },
            )
            .await
            .map(|_| ()),
            QueueOperationKind::Remove => remove_now(state, &operation.target_id).map(|_| ()),
            QueueOperationKind::ImportOptifine => {
                let local_path = operation.local_path.as_deref().ok_or_else(|| {
                    AppError::new(
                        AppErrorCode::ManifestInvalid,
                        "A queued OptiFine import has no local path",
                    )
                })?;
                crate::optifine::import_now(state, Path::new(local_path))?;
                crate::optifine::install_external_components(state).await
            }
        };
        result?;
        queue.operations.retain(|queued| queued.id != operation.id);
        save_queue(state, &queue)?;
    }
    Ok(queue.operations)
}

pub async fn enqueue_optifine(
    state: &AppState,
    source_path: String,
) -> AppResult<ModOperationResult> {
    let _guard = state.operation_lock.lock().await;
    let operation = QueuedOperation {
        id: Uuid::new_v4().to_string(),
        operation_type: QueueOperationKind::ImportOptifine,
        target_id: "local-optifine".to_owned(),
        target_name: "OptiFine 1.8.9".to_owned(),
        created_at: Utc::now().to_rfc3339(),
        status: QueueOperationStatus::Pending,
        retry_count: 0,
        error_message: None,
        version_id: None,
        local_path: Some(source_path),
        allow_beta: false,
    };
    enqueue(state, operation.clone())?;
    Ok(ModOperationResult {
        operation_id: operation.id,
        queued: true,
        installed: Vec::new(),
    })
}

pub fn register_local_mod(state: &AppState, record: InstalledMod) -> AppResult<()> {
    let mut database = load_database(state)?;
    database
        .mods
        .retain(|installed| installed.project_id != record.project_id);
    database.mods.push(record);
    save_database(state, &database)?;
    emit_mods_changed(state)?;
    Ok(())
}

async fn install_now(
    state: &AppState,
    request: InstallModRequest,
) -> AppResult<ModOperationResult> {
    let operation_id = Uuid::new_v4().to_string();
    emit_progress(
        state,
        &operation_id,
        "resolve",
        0.02,
        "Resolving compatible versions and dependencies",
    );
    let plan = modrinth::resolve_plan(
        &state.network,
        &request.project_id,
        request.version_id.as_deref(),
        request.allow_beta,
    )
    .await?;
    let database = load_database(state)?;
    if request.version_id.is_none()
        && database.mods.iter().any(|installed| {
            installed.project_id == request.project_id
                && installed.version_id == plan.public.requested_mod.version_id
        })
    {
        return Ok(ModOperationResult {
            operation_id,
            queued: false,
            installed: list_installed(state)?,
        });
    }
    let transaction = state.paths.staging.join(format!("mods-{operation_id}"));
    fs::create_dir_all(&transaction)
        .map_err(|error| AppError::io("Could not create the mod transaction directory", error))?;
    ensure_inside(&state.paths.staging, &transaction)?;
    let staged_result = stage_plan(state, &operation_id, &transaction, &plan).await;
    let staged = match staged_result {
        Ok(value) => value,
        Err(error) => {
            cleanup_transaction(&state.paths.staging, &transaction);
            return Err(error);
        }
    };
    let result = commit_staged(state, &operation_id, database, staged);
    cleanup_transaction(&state.paths.staging, &transaction);
    result
}

async fn stage_plan(
    state: &AppState,
    operation_id: &str,
    transaction: &Path,
    plan: &ResolvedInstallPlan,
) -> AppResult<Vec<StagedMod>> {
    let total = plan.nodes.len().max(1);
    let required_projects = required_projects(state)?;
    let mut staged = Vec::new();
    for (index, node) in plan.nodes.iter().enumerate() {
        let path = transaction.join(format!("{index}.jar"));
        let sha512 = node.file.hashes.get("sha512").cloned().ok_or_else(|| {
            AppError::new(
                AppErrorCode::ManifestInvalid,
                "The mod provider did not supply SHA-512",
            )
        })?;
        let sha1 = node.file.hashes.get("sha1").cloned();
        let receipt = state
            .network
            .download(
                &node.file.url,
                &path,
                &DownloadExpectation {
                    maximum_size: JAR_LIMIT,
                    expected_size: Some(node.file.size),
                    sha512: Some(sha512.clone()),
                    sha1,
                },
            )
            .await?;
        validate_jar(&path, JAR_LIMIT)?;
        staged.push(StagedMod {
            path,
            record: InstalledMod {
                project_id: node.project_id.clone(),
                id: node.project_id.clone(),
                version_id: node.version_id.clone(),
                name: node.project_name.clone(),
                author: "Modrinth project".to_owned(),
                description: "Installed from the official Modrinth API.".to_owned(),
                icon_url: None,
                version: node.version_number.clone(),
                release_type: ReleaseType::Release,
                downloads: 0,
                updated_at: Utc::now().to_rfc3339(),
                minecraft_version: crate::minecraft::MINECRAFT_VERSION.to_owned(),
                loader: "forge".to_owned(),
                environment: ModEnvironment::Client,
                license: "See Modrinth project".to_owned(),
                file_size: node.file.size,
                dependency_count: node.dependencies.len() as u32,
                trust: ModTrust::FromModrinth,
                compatibility: ModCompatibility::Compatible,
                compatibility_reason: None,
                installed: true,
                installed_version: Some(node.version_number.clone()),
                update_available: false,
                file_name: node.file.filename.clone(),
                sha512: receipt.sha512,
                provider: ModSource::Modrinth,
                required: node.required_dependency || required_projects.contains(&node.project_id),
                installed_at: Utc::now().to_rfc3339(),
                dependencies: node.dependencies.clone(),
                dependents: Vec::new(),
            },
        });
        let progress = 0.08 + (index as f32 + 1.0) / total as f32 * 0.7;
        emit_progress(
            state,
            operation_id,
            "download",
            progress,
            &format!("Downloaded {}", node.project_name),
        );
    }
    Ok(staged)
}

fn commit_staged(
    state: &AppState,
    operation_id: &str,
    mut original_database: InstalledDatabase,
    staged: Vec<StagedMod>,
) -> AppResult<ModOperationResult> {
    let previous_database = original_database.clone();
    let transaction = state.paths.staging.join(format!("rollback-{operation_id}"));
    fs::create_dir_all(&transaction)
        .map_err(|error| AppError::io("Could not create the rollback directory", error))?;
    ensure_inside(&state.paths.staging, &transaction)?;
    let mut backups: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut new_files: Vec<PathBuf> = Vec::new();
    let commit = (|| -> AppResult<()> {
        for (index, item) in staged.iter().enumerate() {
            let destination = state
                .paths
                .mods
                .join(safe_relative_path(&item.record.file_name)?);
            ensure_inside(&state.paths.mods, &destination)?;
            if let Some(previous) = original_database
                .mods
                .iter()
                .find(|installed| installed.project_id == item.record.project_id)
                .cloned()
            {
                let old_path = state
                    .paths
                    .mods
                    .join(safe_relative_path(&previous.file_name)?);
                ensure_inside(&state.paths.mods, &old_path)?;
                if old_path.is_file() {
                    let backup = transaction.join(format!("{index}.bak"));
                    fs::rename(&old_path, &backup).map_err(|error| {
                        AppError::io("Could not back up an installed mod", error)
                    })?;
                    backups.push((backup, old_path));
                }
            }
            if destination.exists() {
                return Err(AppError::new(
                    AppErrorCode::DependencyConflict,
                    "A different installed mod already uses the provider file name",
                )
                .details(destination.to_string_lossy()));
            }
            fs::rename(&item.path, &destination)
                .map_err(|error| AppError::io("Could not atomically install a mod", error))?;
            new_files.push(destination);
            original_database
                .mods
                .retain(|installed| installed.project_id != item.record.project_id);
            original_database.mods.push(item.record.clone());
        }
        save_database(state, &original_database)?;
        Ok(())
    })();
    if let Err(error) = commit {
        for path in new_files.iter().rev() {
            let _ = fs::remove_file(path);
        }
        for (backup, original) in backups.iter().rev() {
            if backup.exists() {
                let _ = fs::rename(backup, original);
            }
        }
        let rollback = save_database(state, &previous_database);
        cleanup_transaction(&state.paths.staging, &transaction);
        if let Err(rollback_error) = rollback {
            return Err(AppError::new(
                AppErrorCode::RollbackFailed,
                "The mod installation and its rollback both failed",
            )
            .details(format!("{error}; {rollback_error}")));
        }
        return Err(error);
    }
    for (backup, _) in backups {
        let _ = fs::remove_file(backup);
    }
    cleanup_transaction(&state.paths.staging, &transaction);
    emit_progress(
        state,
        operation_id,
        "commit",
        1.0,
        "Mod transaction completed",
    );
    emit_mods_changed(state)?;
    Ok(ModOperationResult {
        operation_id: operation_id.to_owned(),
        queued: false,
        installed: list_installed(state)?,
    })
}

fn remove_now(state: &AppState, project_id: &str) -> AppResult<ModOperationResult> {
    if project_id == "private-client-core" {
        return Err(AppError::new(
            AppErrorCode::PermissionDenied,
            "Private Client Core is required and cannot be removed",
        ));
    }
    let operation_id = Uuid::new_v4().to_string();
    let mut database = load_database(state)?;
    let installed = database
        .mods
        .iter()
        .find(|installed| installed.project_id == project_id)
        .cloned()
        .ok_or_else(|| {
            AppError::new(AppErrorCode::ModNotFound, "The installed mod was not found")
        })?;
    if installed.required || required_projects(state)?.contains(project_id) {
        return Err(AppError::new(
            AppErrorCode::PermissionDenied,
            "A required mod cannot be removed",
        ));
    }
    let path = state
        .paths
        .mods
        .join(safe_relative_path(&installed.file_name)?);
    ensure_inside(&state.paths.mods, &path)?;
    let backup = state
        .paths
        .staging
        .join(format!("remove-{operation_id}.bak"));
    if path.is_file() {
        fs::rename(&path, &backup)
            .map_err(|error| AppError::io("Could not stage the mod removal", error))?;
    }
    database.mods.retain(|entry| entry.project_id != project_id);
    if let Err(error) = save_database(state, &database) {
        if backup.exists() {
            let _ = fs::rename(&backup, &path);
        }
        return Err(error);
    }
    if backup.exists() {
        fs::remove_file(&backup)
            .map_err(|error| AppError::io("Could not finalize the mod removal", error))?;
    }
    emit_mods_changed(state)?;
    Ok(ModOperationResult {
        operation_id,
        queued: false,
        installed: list_installed(state)?,
    })
}

pub(crate) fn remove_local_mod_now(
    state: &AppState,
    project_id: &str,
) -> AppResult<ModOperationResult> {
    remove_now(state, project_id)
}

fn ensure_installed(state: &AppState, project_id: &str) -> AppResult<()> {
    if load_database(state)?
        .mods
        .iter()
        .any(|installed| installed.project_id == project_id)
    {
        Ok(())
    } else {
        Err(AppError::new(
            AppErrorCode::ModNotFound,
            "The requested mod is not installed",
        ))
    }
}

fn reject_required_mod_mutation(state: &AppState, project_id: &str) -> AppResult<()> {
    if required_projects(state)?.contains(project_id)
        || load_database(state)?
            .mods
            .iter()
            .any(|installed| installed.project_id == project_id && installed.required)
    {
        return Err(AppError::new(
            AppErrorCode::PermissionDenied,
            "A required Private Client mod cannot be changed manually",
        ));
    }
    Ok(())
}

fn load_database(state: &AppState) -> AppResult<InstalledDatabase> {
    if !state.paths.installed_mods.exists() {
        return Ok(InstalledDatabase::default());
    }
    let database: InstalledDatabase = read_json(&state.paths.installed_mods)?;
    if database.schema_version != 1 {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "The installed mod database schema is unsupported",
        ));
    }
    let mut seen = BTreeSet::new();
    for installed in &database.mods {
        validate_identifier(&installed.project_id, "Installed mod project ID")?;
        if !seen.insert(installed.project_id.clone()) {
            return Err(AppError::new(
                AppErrorCode::ManifestInvalid,
                "The installed mod database contains duplicate projects",
            ));
        }
        let _ = safe_relative_path(&installed.file_name)?;
    }
    Ok(database)
}

fn save_database(state: &AppState, database: &InstalledDatabase) -> AppResult<()> {
    atomic_write_json(&state.paths.installed_mods, database)
}

fn load_queue(state: &AppState) -> AppResult<OperationQueue> {
    if !state.paths.operation_queue.exists() {
        return Ok(OperationQueue::default());
    }
    let queue: OperationQueue = read_json(&state.paths.operation_queue)?;
    if queue.schema_version != 1 || queue.operations.len() > 256 {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "The pending operation queue is invalid",
        ));
    }
    Ok(queue)
}

fn save_queue(state: &AppState, queue: &OperationQueue) -> AppResult<()> {
    atomic_write_json(&state.paths.operation_queue, queue)
}

fn enqueue(state: &AppState, operation: QueuedOperation) -> AppResult<()> {
    let mut queue = load_queue(state)?;
    if queue.operations.len() >= 256 {
        return Err(AppError::new(
            AppErrorCode::DependencyConflict,
            "The pending mod operation queue is full",
        ));
    }
    queue.operations.push(operation);
    save_queue(state, &queue)
}

fn append_build_records(state: &AppState, mods: &mut Vec<InstalledMod>) -> AppResult<()> {
    if mods
        .iter()
        .any(|installed| installed.project_id == REQUIRED_PRIVATE_CLIENT_CORE_ID)
    {
        return Ok(());
    }
    let path = state.paths.mods.join(REQUIRED_PRIVATE_CLIENT_CORE_FILE);
    if path.is_file() {
        let (sha512, _, _) = hash_file(&path)?;
        mods.push(InstalledMod {
            id: REQUIRED_PRIVATE_CLIENT_CORE_ID.to_owned(),
            project_id: REQUIRED_PRIVATE_CLIENT_CORE_ID.to_owned(),
            version_id: format!("{}-1.0.0", REQUIRED_PRIVATE_CLIENT_CORE_ID),
            name: "Private Client Core".to_owned(),
            author: "Private Client".to_owned(),
            description: "A consolidated bundle of the core client modules.".to_owned(),
            icon_url: None,
            version: "1.0.0".to_owned(),
            release_type: ReleaseType::Release,
            downloads: 0,
            updated_at: Utc::now().to_rfc3339(),
            minecraft_version: crate::minecraft::MINECRAFT_VERSION.to_owned(),
            loader: "forge".to_owned(),
            environment: ModEnvironment::Client,
            license: "Private Client Source License".to_owned(),
            file_size: path.metadata().map(|metadata| metadata.len()).unwrap_or(0),
            dependency_count: 0,
            trust: ModTrust::Verified,
            compatibility: ModCompatibility::Compatible,
            compatibility_reason: None,
            installed: true,
            installed_version: Some("1.0.0".to_owned()),
            update_available: false,
            file_name: REQUIRED_PRIVATE_CLIENT_CORE_FILE.to_owned(),
            sha512,
            provider: ModSource::PrivateClient,
            required: true,
            installed_at: Utc::now().to_rfc3339(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
        });
    }
    Ok(())
}

fn required_projects(_state: &AppState) -> AppResult<BTreeSet<String>> {
    let manifest = load_required_manifest()?;
    let mut projects = BTreeSet::new();
    for entry in manifest.mods.into_iter().filter(|entry| entry.required) {
        let project_id = entry.project_id.unwrap_or(entry.id);
        validate_identifier(&project_id, "Required mod project ID")?;
        projects.insert(project_id);
    }
    Ok(projects)
}

fn load_required_manifest() -> AppResult<RequiredModsManifest> {
    let manifest: RequiredModsManifest = serde_json::from_str(EMBEDDED_REQUIRED_MODS_JSON)
        .map_err(|error| AppError::json("The embedded required-mod manifest is invalid", error))?;
    if manifest.schema_version != 1
        || manifest.minecraft_version != crate::minecraft::MINECRAFT_VERSION
        || manifest.loader != "forge"
        || manifest.mods.len() > 32
    {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "The bundled required-mod manifest is incompatible",
        ));
    }
    Ok(manifest)
}

fn emit_mods_changed(state: &AppState) -> AppResult<()> {
    let _ = state.app.emit(
        EVENT_MODS_CHANGED,
        ModsChangedEvent {
            reason: "changed".to_owned(),
            project_id: None,
        },
    );
    Ok(())
}

fn emit_progress(state: &AppState, operation_id: &str, stage: &str, progress: f32, message: &str) {
    let event = ProgressEvent {
        operation_id: operation_id.to_owned(),
        target_id: "mods".to_owned(),
        phase: stage.to_owned(),
        progress: progress.clamp(0.0, 1.0) * 100.0,
        message: message.to_owned(),
    };
    let _ = state.app.emit(EVENT_OPERATION_PROGRESS, event);
}

fn validate_uuid(value: &str) -> AppResult<()> {
    Uuid::parse_str(value).map(|_| ()).map_err(|error| {
        AppError::new(AppErrorCode::InvalidInput, "The operation ID is invalid")
            .details(error.to_string())
    })
}

fn cleanup_transaction(root: &Path, transaction: &Path) {
    if ensure_inside(root, transaction).is_ok() {
        let _ = fs::remove_dir_all(transaction);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        validate_pinned_ias_entry, InstalledDatabase, OperationQueue, RequiredModsManifest,
        REQUIRED_IAS_ID, REQUIRED_PRIVATE_CLIENT_CORE_ID,
    };

    #[test]
    fn local_databases_start_with_versioned_schemas() {
        assert_eq!(InstalledDatabase::default().schema_version, 1);
        assert_eq!(OperationQueue::default().schema_version, 1);
        assert!(OperationQueue::default().operations.is_empty());
    }

    #[test]
    fn bundled_manifest_supports_consolidated_core_and_pins_external_ias(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let manifest: RequiredModsManifest = serde_json::from_str(include_str!(
            "../../../../manifests/mods/required-mods.json"
        ))?;

        // Verify the consolidated private-client-core entry
        let core = manifest
            .mods
            .iter()
            .find(|e| e.id == REQUIRED_PRIVATE_CLIENT_CORE_ID)
            .ok_or("missing private-client-core")?;
        assert_eq!(core.provider, "private-client-build");
        assert!(core.project_id.is_none());
        assert!(core.required);
        assert!(!core.removable);

        // Verify the pinned IAS entry
        let ias = manifest
            .mods
            .iter()
            .find(|entry| entry.id == REQUIRED_IAS_ID)
            .ok_or("missing IAS")?;
        assert!(validate_pinned_ias_entry(ias).is_ok());
        Ok(())
    }
}
