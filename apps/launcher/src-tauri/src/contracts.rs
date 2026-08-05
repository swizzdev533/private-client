//! Stable IPC contract shared with `apps/launcher/src/types/contracts.ts`.
//!
//! Public payloads intentionally contain no access token, refresh token,
//! password, cookie, device identifier, or telemetry identifier.

use serde::{Deserialize, Serialize};

pub const EVENT_LAUNCH_STATE: &str = "launcher://launch-state";
pub const EVENT_PROFILE_UPDATED: &str = "launcher://profile-updated";
pub const EVENT_MODS_CHANGED: &str = "launcher://mods-changed";
pub const EVENT_OPERATION_PROGRESS: &str = "launcher://operation-progress";

/// Strict `major.minor.patch` used to compare the installed launcher against an
/// advertised update. Parsing is deliberately narrow so an unbounded tag such as
/// `latest` can never satisfy a version comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn parse(raw: &str) -> Option<Self> {
        let mut parts = raw.trim().split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

/// Result of an update check. Contains no host, path, or identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub available: bool,
    pub current_version: String,
    pub available_version: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
}

impl UpdateStatus {
    pub fn current(current_version: String) -> Self {
        Self {
            available: false,
            current_version,
            available_version: None,
            notes: None,
            published_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PathSnapshot {
    pub data_root: String,
    pub instance_root: String,
    pub logs_directory: String,
    pub mods_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalProfile {
    pub schema_version: u32,
    pub username: String,
    pub uuid: String,
    pub skin_path: Option<String>,
    pub skin_model: SkinModel,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SkinModel {
    #[default]
    Classic,
    Slim,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JavaCandidate {
    pub executable: String,
    pub version: String,
    pub major: u16,
    pub architecture: String,
    pub compatible: bool,
    pub source: JavaSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum JavaSource {
    Configured,
    JavaHome,
    Path,
    ProgramFiles,
    Managed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JavaDetection {
    pub selected: Option<JavaCandidate>,
    pub candidates: Vec<JavaCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JavaConfig {
    pub executable: String,
    pub minimum_memory_mb: u32,
    pub maximum_memory_mb: u32,
}

impl Default for JavaConfig {
    fn default() -> Self {
        Self {
            executable: String::new(),
            minimum_memory_mb: 1024,
            maximum_memory_mb: 4096,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LaunchPhase {
    #[default]
    Idle,
    Validating,
    CheckingRuntime,
    PreparingInstance,
    VerifyingGameFiles,
    InstallingGameFiles,
    VerifyingForge,
    InstallingForge,
    CheckingRequiredMods,
    ApplyingPendingChanges,
    BuildingLaunchCommand,
    Launching,
    Running,
    Stopping,
    Exited,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchState {
    pub state: LaunchPhase,
    pub message: String,
    pub progress: Option<f32>,
    pub can_cancel: bool,
    pub error_id: Option<String>,
    pub log_path: Option<String>,
    #[serde(skip)]
    pub pid: Option<u32>,
    #[serde(skip)]
    pub started_at: Option<String>,
    #[serde(skip)]
    pub finished_at: Option<String>,
    #[serde(skip)]
    pub exit_code: Option<i32>,
    #[serde(skip)]
    pub crash_kind: Option<CrashKind>,
}

impl Default for LaunchState {
    fn default() -> Self {
        Self {
            state: LaunchPhase::Idle,
            message: "Instancja jest gotowa".to_owned(),
            progress: None,
            can_cancel: false,
            error_id: None,
            log_path: None,
            pid: None,
            started_at: None,
            finished_at: None,
            exit_code: None,
            crash_kind: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CrashKind {
    CleanExit,
    UserTerminated,
    OutOfMemory,
    MissingLibrary,
    ModConflict,
    ForgeCrash,
    JvmError,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub operation_id: String,
    pub target_id: String,
    pub phase: String,
    pub message: String,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModsChangedEvent {
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSnapshot {
    pub installed: bool,
    pub minecraft_ready: bool,
    pub forge_ready: bool,
    pub version: String,
    pub forge_version: String,
    pub last_verified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    pub installed: bool,
    pub healthy: bool,
    pub minecraft_version: String,
    pub forge_version: String,
    pub java_label: Option<String>,
    pub pending_operations: u32,
    pub last_played_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSnapshot {
    pub profile: Option<LocalProfile>,
    pub launch: LaunchState,
    pub settings: LauncherSettings,
    pub instance: InstanceSummary,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    #[serde(default = "schema_one")]
    pub schema_version: u32,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default = "default_memory_min")]
    pub memory_min_mb: u32,
    #[serde(default = "default_memory_max")]
    pub memory_max_mb: u32,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default = "default_false")]
    pub auto_update_checks: bool,
    #[serde(default = "default_download_concurrency")]
    pub download_concurrency: u8,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            schema_version: schema_one(),
            java_path: None,
            memory_min_mb: default_memory_min(),
            memory_max_mb: default_memory_max(),
            reduced_motion: false,
            auto_update_checks: false,
            download_concurrency: default_download_concurrency(),
        }
    }
}

fn schema_one() -> u32 {
    1
}

fn default_false() -> bool {
    false
}

fn default_memory_min() -> u32 {
    1024
}

fn default_memory_max() -> u32 {
    4096
}

fn default_download_concurrency() -> u8 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrepareInstanceRequest {
    #[serde(default)]
    pub repair: bool,
    pub java_executable: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub minimum_memory_mb: u32,
    pub maximum_memory_mb: u32,
    pub width: u32,
    pub height: u32,
}

impl LaunchRequest {
    pub fn from_settings(settings: &LauncherSettings) -> Self {
        Self {
            minimum_memory_mb: settings.memory_min_mb,
            maximum_memory_mb: settings.memory_max_mb,
            width: 1280,
            height: 720,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModSearchSort {
    Relevance,
    Downloads,
    Updated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModSearchTrust {
    All,
    Verified,
    Modrinth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModSearchRequest {
    pub query: String,
    pub sort: ModSearchSort,
    pub trust: ModSearchTrust,
    pub page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModTrust {
    Verified,
    FromModrinth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModCompatibility {
    Compatible,
    Experimental,
    LicenseReview,
    Incompatible,
    DownloadUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseType {
    Release,
    Beta,
    Alpha,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModEnvironment {
    Client,
    ClientAndServer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModSummary {
    pub id: String,
    pub project_id: String,
    pub version_id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub version: String,
    pub release_type: ReleaseType,
    pub downloads: u64,
    pub updated_at: String,
    pub minecraft_version: String,
    pub loader: String,
    pub environment: ModEnvironment,
    pub license: String,
    pub file_size: u64,
    pub dependency_count: u32,
    pub trust: ModTrust,
    pub compatibility: ModCompatibility,
    pub compatibility_reason: Option<String>,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub update_available: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModSearchResponse {
    pub query: String,
    pub results: Vec<ModSummary>,
    pub page: u32,
    pub has_more: bool,
    pub from_cache: bool,
    pub offline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallModRequest {
    pub project_id: String,
    pub version_id: Option<String>,
    #[serde(default)]
    pub allow_beta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateModRequest {
    pub project_id: String,
    #[serde(default)]
    pub allow_beta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub id: String,
    pub project_id: String,
    pub version_id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub version: String,
    pub release_type: ReleaseType,
    pub downloads: u64,
    pub updated_at: String,
    pub minecraft_version: String,
    pub loader: String,
    pub environment: ModEnvironment,
    pub license: String,
    pub file_size: u64,
    pub dependency_count: u32,
    pub trust: ModTrust,
    pub compatibility: ModCompatibility,
    pub compatibility_reason: Option<String>,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub update_available: bool,
    pub required: bool,
    pub file_name: String,
    pub sha512: String,
    pub installed_at: String,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub provider: ModSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModSource {
    #[serde(rename = "modrinth")]
    Modrinth,
    #[serde(rename = "local-import")]
    LocalImport,
    #[serde(rename = "private-client")]
    PrivateClient,
    #[serde(rename = "github")]
    Github,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueueOperationKind {
    Install,
    Update,
    Remove,
    ImportOptifine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueueOperationStatus {
    Pending,
    Running,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueuedOperation {
    pub id: String,
    #[serde(rename = "type")]
    pub operation_type: QueueOperationKind,
    pub target_id: String,
    pub target_name: String,
    pub created_at: String,
    pub status: QueueOperationStatus,
    pub retry_count: u32,
    pub error_message: Option<String>,
    pub version_id: Option<String>,
    pub local_path: Option<String>,
    pub allow_beta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingOperation {
    pub id: String,
    #[serde(rename = "type")]
    pub operation_type: QueueOperationKind,
    pub target_id: String,
    pub target_name: String,
    pub created_at: String,
    pub status: QueueOperationStatus,
    pub retry_count: u32,
    pub error_message: Option<String>,
}

impl From<&QueuedOperation> for PendingOperation {
    fn from(operation: &QueuedOperation) -> Self {
        Self {
            id: operation.id.clone(),
            operation_type: operation.operation_type.clone(),
            target_id: operation.target_id.clone(),
            target_name: operation.target_name.clone(),
            created_at: operation.created_at.clone(),
            status: operation.status.clone(),
            retry_count: operation.retry_count,
            error_message: operation.error_message.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModOperationResult {
    pub operation_id: String,
    pub queued: bool,
    pub installed: Vec<InstalledMod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModInstallPlan {
    pub requested_mod: ModInstallPlanItem,
    pub dependencies: Vec<ModInstallPlanItem>,
    pub expected_disk_usage: u64,
    pub files_to_replace: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModInstallPlanItem {
    pub project_id: String,
    pub version_id: String,
    pub name: String,
    pub version: String,
    pub file_size: u64,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoveModRequest {
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptifineRequest {
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub ok: bool,
    pub message: String,
    pub queued: bool,
}

impl CommandResult {
    pub fn completed(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: message.into(),
            queued: false,
        }
    }

    pub fn queued(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: message.into(),
            queued: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandResult, LaunchPhase, LaunchState};

    #[test]
    fn launch_contract_serializes_frontend_enum_and_percent_progress(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = LaunchState {
            state: LaunchPhase::InstallingGameFiles,
            message: "Installing".to_owned(),
            progress: Some(42.5),
            can_cancel: true,
            ..LaunchState::default()
        };
        let value = serde_json::to_value(state)?;
        assert_eq!(value["state"], "INSTALLING_GAME_FILES");
        assert_eq!(value["progress"], 42.5);
        assert!(value.get("phase").is_none());
        assert!(value.get("pid").is_none());
        Ok(())
    }

    #[test]
    fn command_result_has_exact_public_shape() -> Result<(), Box<dyn std::error::Error>> {
        let value = serde_json::to_value(CommandResult::queued("queued"))?;
        assert_eq!(value["ok"], true);
        assert_eq!(value["message"], "queued");
        assert_eq!(value["queued"], true);
        assert_eq!(value.as_object().map(serde_json::Map::len), Some(3));
        Ok(())
    }
}
