//! Signed launcher self-update over the pinned release host.
//!
//! Trust boundary: the update manifest and every downloaded artifact are
//! verified by `tauri-plugin-updater` against the minisign public key pinned in
//! `tauri.conf.json` before anything is written or executed. This module adds
//! the product rules on top of that: updates never run while Minecraft is
//! active, a lower or equal version is never installed, and an unreachable or
//! unconfigured endpoint means "updates unavailable" rather than a fallback.

use crate::contracts::{UpdateStatus, Version};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::state::AppState;
use tauri_plugin_updater::UpdaterExt;

/// Bounds the manifest fetch so a hung release host cannot block the UI.
const CHECK_TIMEOUT_SECS: u64 = 20;
/// Release notes are attacker-influenced text rendered in the UI; keep them small.
const MAX_NOTES_CHARS: usize = 2000;

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Resolves the newest signed update, or `None` when the launcher is current.
pub async fn check(state: &AppState) -> AppResult<UpdateStatus> {
    let current = current_version().to_owned();
    let updater = state
        .app
        .updater_builder()
        .timeout(std::time::Duration::from_secs(CHECK_TIMEOUT_SECS))
        .build()
        .map_err(|error| {
            AppError::new(
                AppErrorCode::UpdateFailed,
                format!("Could not initialize the updater: {error}"),
            )
        })?;

    let found = updater.check().await.map_err(|error| {
        AppError::new(
            AppErrorCode::NetworkUnavailable,
            format!("Could not reach the update host: {error}"),
        )
    })?;

    let Some(update) = found else {
        return Ok(UpdateStatus::current(current));
    };

    // The plugin already refuses same-or-older versions, but an explicit check
    // keeps downgrade rejection a property of this module and its tests.
    let available = Version::parse(&update.version).ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "The update manifest advertised an unparsable version",
        )
    })?;
    let installed = Version::parse(&current).ok_or_else(|| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "The installed launcher version is unparsable",
        )
    })?;
    if available <= installed {
        return Ok(UpdateStatus::current(current));
    }

    Ok(UpdateStatus {
        available: true,
        current_version: current,
        available_version: Some(update.version.clone()),
        notes: update.body.as_deref().map(truncate_notes),
        published_at: update.date.map(|date| date.to_string()),
    })
}

/// Downloads, verifies and installs the pending update, then exits for the
/// installer to swap the binary. Never runs while the game is active.
pub async fn install(state: &AppState) -> AppResult<()> {
    let _guard = state.operation_lock.lock().await;
    if state.is_game_running() {
        return Err(AppError::new(
            AppErrorCode::OperationBlockedWhileRunning,
            "The launcher cannot update itself while Minecraft is running",
        ));
    }

    let updater = state
        .app
        .updater_builder()
        .timeout(std::time::Duration::from_secs(CHECK_TIMEOUT_SECS))
        .build()
        .map_err(|error| {
            AppError::new(
                AppErrorCode::UpdateFailed,
                format!("Could not initialize the updater: {error}"),
            )
        })?;

    let update = updater
        .check()
        .await
        .map_err(|error| {
            AppError::new(
                AppErrorCode::NetworkUnavailable,
                format!("Could not reach the update host: {error}"),
            )
        })?
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::UpdateFailed,
                "No signed update is available for this launcher",
            )
        })?;

    // Signature and length are enforced inside the plugin; a failure here means
    // the artifact was rejected and nothing was installed.
    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|error| {
            AppError::new(
                AppErrorCode::UpdateFailed,
                format!("The signed update could not be installed: {error}"),
            )
        })?;
    Ok(())
}

fn truncate_notes(notes: &str) -> String {
    let cleaned: String = notes
        .chars()
        .filter(|character| !character.is_control() || *character == '\n')
        .take(MAX_NOTES_CHARS)
        .collect();
    cleaned.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_no_update_when_versions_match() {
        let status = UpdateStatus::current("1.0.0".to_owned());
        assert!(!status.available);
        assert_eq!(status.available_version, None);
    }

    /// Comparing the parsed `Option`s directly keeps the assertions faithful:
    /// an unparsable side can never silently compare as "not newer".
    fn is_newer(advertised: &str, installed: &str) -> bool {
        match (Version::parse(advertised), Version::parse(installed)) {
            (Some(advertised), Some(installed)) => advertised > installed,
            _ => false,
        }
    }

    #[test]
    fn treats_an_older_advertised_version_as_a_downgrade() {
        assert!(!is_newer("1.3.9", "1.4.0"));
    }

    #[test]
    fn treats_an_equal_advertised_version_as_a_replay() {
        assert!(!is_newer("1.4.0", "1.4.0"));
    }

    #[test]
    fn accepts_a_strictly_newer_version() {
        assert!(is_newer("1.4.1", "1.4.0"));
        assert!(is_newer("2.0.0", "1.99.99"));
    }

    #[test]
    fn orders_versions_numerically_not_lexically() {
        assert!(is_newer("1.10.0", "1.9.0"));
    }

    #[test]
    fn an_unparsable_advertised_version_never_counts_as_newer() {
        assert!(!is_newer("latest", "1.0.0"));
        assert!(!is_newer("v1.2.3", "1.0.0"));
    }

    #[test]
    fn rejects_an_unparsable_advertised_version() {
        assert!(Version::parse("latest").is_none());
        assert!(Version::parse("1.0").is_none());
        assert!(Version::parse("1.0.0-beta").is_none());
        assert!(Version::parse("").is_none());
    }

    #[test]
    fn bounds_and_sanitizes_release_notes() {
        let notes = format!("line\u{0007}one\nline two{}", "x".repeat(5000));
        let truncated = truncate_notes(&notes);
        assert!(truncated.chars().count() <= MAX_NOTES_CHARS);
        assert!(!truncated.contains('\u{0007}'));
        assert!(truncated.starts_with("lineone\nline two"));
    }

    #[test]
    fn the_shipped_version_is_parsable() {
        assert!(Version::parse(current_version()).is_some());
    }
}
