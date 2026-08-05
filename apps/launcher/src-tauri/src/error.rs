use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::Path;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum AppErrorCode {
    InvalidInput,
    Io,
    Json,
    JavaNotFound,
    JavaIncompatible,
    JavaArchitectureMismatch,
    RuntimeDownloadFailed,
    InstanceCorrupted,
    MinecraftMetadataInvalid,
    ForgeInstallationFailed,
    ManifestInvalid,
    ModNotFound,
    ModIncompatible,
    ModAlreadyInstalled,
    DependencyConflict,
    DependencyCycle,
    DownloadFailed,
    DownloadTooLarge,
    DownloadTimedOut,
    UnsafeRedirect,
    UntrustedHost,
    HashMismatch,
    JarValidationFailed,
    PathTraversalDetected,
    SymlinkDetected,
    InsufficientDiskSpace,
    InsufficientMemory,
    GameAlreadyRunning,
    GameNotRunning,
    LaunchFailed,
    GameCrashed,
    ProfileCacheInvalid,
    ProfileWriteFailed,
    PermissionDenied,
    NetworkUnavailable,
    UpdateFailed,
    RollbackFailed,
    OperationQueued,
    OperationBlockedWhileRunning,
    SingleInstanceViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    #[serde(rename = "id")]
    pub code: AppErrorCode,
    pub title: Box<str>,
    pub message: Box<str>,
    #[serde(rename = "resolution")]
    pub recovery: Option<Box<str>>,
    pub log_path: Option<Box<str>>,
    #[serde(skip)]
    pub details: Option<Box<str>>,
}

impl AppError {
    pub fn new(code: AppErrorCode, message: impl Into<String>) -> Self {
        let (title, recovery) = defaults(&code);
        Self {
            code,
            title: Box::<str>::from(title),
            message: message.into().into_boxed_str(),
            recovery: recovery.map(Box::<str>::from),
            log_path: None,
            details: None,
        }
    }

    pub fn recovery(mut self, recovery: impl Into<String>) -> Self {
        self.recovery = Some(recovery.into().into_boxed_str());
        self
    }

    pub fn details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into().into_boxed_str());
        self
    }

    pub fn with_log(mut self, path: &Path) -> Self {
        self.log_path = Some(path.to_string_lossy().into_owned().into_boxed_str());
        self
    }

    pub fn io(context: impl Into<String>, error: impl Display) -> Self {
        Self::new(AppErrorCode::Io, context).details(error.to_string())
    }

    pub fn json(context: impl Into<String>, error: impl Display) -> Self {
        Self::new(AppErrorCode::Json, context).details(error.to_string())
    }
}

fn defaults(code: &AppErrorCode) -> (&'static str, Option<&'static str>) {
    match code {
        AppErrorCode::RuntimeDownloadFailed => (
            "Could not prepare Java 8",
            Some("Check your internet connection and press PLAY again, or select java.exe in settings."),
        ),
        AppErrorCode::JavaNotFound
        | AppErrorCode::JavaIncompatible
        | AppErrorCode::JavaArchitectureMismatch => (
            "Invalid Java environment",
            Some("Select a 64-bit Java 8 runtime in the launcher settings."),
        ),
        AppErrorCode::InstanceCorrupted
        | AppErrorCode::MinecraftMetadataInvalid
        | AppErrorCode::ForgeInstallationFailed
        | AppErrorCode::ManifestInvalid => (
            "The game instance needs repair",
            Some("Restart the launcher and prepare the instance again."),
        ),
        AppErrorCode::DownloadFailed
        | AppErrorCode::DownloadTooLarge
        | AppErrorCode::DownloadTimedOut
        | AppErrorCode::NetworkUnavailable
        | AppErrorCode::UpdateFailed => (
            "Could not download the file",
            Some("Check your internet connection and try again."),
        ),
        AppErrorCode::UnsafeRedirect
        | AppErrorCode::UntrustedHost
        | AppErrorCode::HashMismatch
        | AppErrorCode::JarValidationFailed
        | AppErrorCode::PathTraversalDetected
        | AppErrorCode::SymlinkDetected => (
            "An unsafe file was blocked",
            Some("Do not bypass verification; choose the original file from a trusted source."),
        ),
        AppErrorCode::ModNotFound
        | AppErrorCode::ModIncompatible
        | AppErrorCode::ModAlreadyInstalled
        | AppErrorCode::DependencyConflict
        | AppErrorCode::DependencyCycle => (
            "Could not apply the mod",
            Some("Choose a release compatible with Minecraft 1.8.9 and Forge."),
        ),
        AppErrorCode::GameAlreadyRunning
        | AppErrorCode::GameNotRunning
        | AppErrorCode::LaunchFailed
        | AppErrorCode::GameCrashed
        | AppErrorCode::OperationBlockedWhileRunning => (
            "Could not start the game",
            Some("Close the running game, check the logs and try again."),
        ),
        AppErrorCode::ProfileCacheInvalid | AppErrorCode::ProfileWriteFailed => (
            "The local profile is invalid",
            Some("Sign in again inside the game to rebuild the profile."),
        ),
        AppErrorCode::RollbackFailed => (
            "Could not roll back the change",
            Some("Do not start the game; check the local launcher logs."),
        ),
        AppErrorCode::InsufficientMemory => (
            "Not enough free memory",
            Some("Lower the maximum game RAM in settings."),
        ),
        AppErrorCode::InsufficientDiskSpace => (
            "Not enough disk space",
            Some("Free at least 2 GiB on the local drive."),
        ),
        AppErrorCode::PermissionDenied | AppErrorCode::SingleInstanceViolation => (
            "The launcher was denied access",
            Some("Close the other launcher window and check the data directory permissions."),
        ),
        AppErrorCode::InvalidInput
        | AppErrorCode::Io
        | AppErrorCode::Json
        | AppErrorCode::OperationQueued => (
            "The operation failed",
            Some("Check the values you entered and try again."),
        ),
    }
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::io("A local file operation failed", value)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::json("A local JSON document is invalid", value)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        let code = if value.is_timeout() {
            AppErrorCode::DownloadTimedOut
        } else if value.is_connect() {
            AppErrorCode::NetworkUnavailable
        } else {
            AppErrorCode::DownloadFailed
        };
        Self::new(code, "The network request failed").details(value.to_string())
    }
}

impl From<url::ParseError> for AppError {
    fn from(value: url::ParseError) -> Self {
        Self::new(AppErrorCode::InvalidInput, "The URL is invalid").details(value.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(value: zip::result::ZipError) -> Self {
        Self::new(
            AppErrorCode::JarValidationFailed,
            "The JAR archive is invalid",
        )
        .details(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{AppError, AppErrorCode};
    use std::path::Path;

    #[test]
    fn serializes_frontend_domain_error_shape() -> Result<(), Box<dyn std::error::Error>> {
        let error = AppError::new(AppErrorCode::JavaNotFound, "Java was not found")
            .details("sensitive diagnostic")
            .with_log(Path::new(r"C:\Private Client\logs\launcher.log"));
        let value = serde_json::to_value(error)?;
        assert_eq!(value["id"], "JavaNotFound");
        assert_eq!(value["title"], "Invalid Java environment");
        assert_eq!(value["message"], "Java was not found");
        assert!(value["resolution"].is_string());
        assert!(value["logPath"].is_string());
        assert!(value.get("details").is_none());
        Ok(())
    }
}
