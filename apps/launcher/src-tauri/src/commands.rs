use crate::contracts::{
    CommandResult, InstallModRequest, InstalledMod, InstanceSummary, LauncherSettings,
    LauncherSnapshot, ModInstallPlan, ModSearchRequest, ModSearchResponse, PendingOperation,
    RemoveModRequest, UpdateModRequest, UpdateStatus,
};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::state::AppState;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn get_launcher_snapshot(state: State<'_, Arc<AppState>>) -> AppResult<LauncherSnapshot> {
    let settings = state.config.load_settings()?;
    let profile = crate::profile::read(&state.paths)?;
    let instance = crate::minecraft::snapshot(&state.paths);
    let java_label = crate::java::detect(&state.config, &state.paths)
        .await?
        .selected
        .map(|java| format!("Java {} · {}", java.version, java.architecture));
    let pending_operations =
        u32::try_from(crate::mods::list_pending(&state)?.len()).unwrap_or(u32::MAX);
    Ok(LauncherSnapshot {
        profile,
        launch: state.launch_state(),
        settings,
        instance: InstanceSummary {
            installed: instance.installed,
            healthy: instance.minecraft_ready && instance.forge_ready,
            minecraft_version: crate::minecraft::MINECRAFT_VERSION.to_owned(),
            forge_version: crate::minecraft::FORGE_DISPLAY_VERSION.to_owned(),
            java_label,
            pending_operations,
            last_played_at: state.launch_state().finished_at,
        },
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        channel: crate::channel::CHANNEL.as_str().to_owned(),
    })
}

#[tauri::command]
pub async fn launch_game(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    let settings = state.config.load_settings()?;
    let request = crate::contracts::LaunchRequest::from_settings(&settings);
    match crate::launcher::launch(Arc::clone(state.inner()), request).await {
        Ok(_) => Ok(CommandResult::completed("Minecraft has started")),
        Err(error) => {
            let error = error.with_log(state.logger.path());
            let mut launch = state.launch_state();
            launch.state = crate::contracts::LaunchPhase::Failed;
            launch.message = error.message.to_string();
            launch.progress = None;
            launch.can_cancel = false;
            launch.error_id = Some(format!("{:?}", error.code));
            launch.log_path = error.log_path.as_deref().map(str::to_owned);
            state.set_launch_state(launch);
            Err(error)
        }
    }
}

#[tauri::command]
pub fn cancel_launch(state: State<'_, Arc<AppState>>) -> CommandResult {
    crate::launcher::cancel(&state)
}

#[tauri::command]
pub async fn stop_game(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    crate::launcher::stop(&state).await
}

#[tauri::command]
pub async fn focus_game_window(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    crate::launcher::focus_game(&state).await
}

#[tauri::command]
pub fn save_launcher_settings(
    state: State<'_, Arc<AppState>>,
    settings: LauncherSettings,
) -> AppResult<LauncherSettings> {
    state.config.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn open_logs_directory(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    crate::diagnostics::open_logs(&state)?;
    Ok(CommandResult::completed("Opened the logs directory"))
}

#[tauri::command]
pub fn export_logs(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    let exports = state.paths.logs.join("exports");
    fs::create_dir_all(&exports)
        .map_err(|error| AppError::io("Could not create the log export directory", error))?;
    let destination = exports.join(format!(
        "private-client-logs-{}.zip",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    let destination_text = destination.to_string_lossy().into_owned();
    let exported = crate::diagnostics::export_logs(&state, &destination_text)?;
    Ok(CommandResult::completed(format!(
        "Exported logs: {exported}"
    )))
}

#[tauri::command]
pub async fn search_modrinth(
    state: State<'_, Arc<AppState>>,
    request: ModSearchRequest,
) -> AppResult<ModSearchResponse> {
    let mut response = crate::modrinth::search(&state.network, &request).await?;
    let installed = crate::mods::list_installed(&state)?;
    for result in &mut response.results {
        if let Some(current) = installed
            .iter()
            .find(|candidate| candidate.project_id == result.project_id)
        {
            result.installed = true;
            result.installed_version = Some(current.version.clone());
            result.update_available = current.version_id != result.version_id;
            result.required = current.required;
        }
    }
    Ok(response)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_mod_install_plan(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<ModInstallPlan> {
    crate::mods::get_install_plan(
        &state,
        &InstallModRequest {
            project_id,
            version_id: None,
            allow_beta: true,
        },
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn install_mod(
    state: State<'_, Arc<AppState>>,
    project_id: String,
    version_id: String,
) -> AppResult<CommandResult> {
    let result = crate::mods::install(
        &state,
        InstallModRequest {
            project_id,
            version_id: Some(version_id),
            allow_beta: true,
        },
    )
    .await?;
    Ok(operation_result("The mod was installed", result.queued))
}

/// Every project id that "remove Private Pack" must clean up.
///
/// This is a superset of `optifine::CURATED_PACK_COMPONENTS`: it also covers the
/// separately imported components (OptiFine, HitDelayFix, animations,
/// fullbright) and legacy ids that older installs may still carry, so a pack
/// removal leaves nothing behind.
pub(crate) const REMOVABLE_PACK_COMPONENTS: &[&str] = &[
    "local-optifine",
    "external-hitdelayfix",
    "4Hfmgaef",
    "8L5i5hyX",
    "NNAgCjsB",
    "jupr7Bf5",
    "5uJtFIcj",
    "YknNc5nN",
    "BpzUOKOJ",
    "xgpAkTGi",
    "w6x8nHjH",
    "oCBQFmrZ",
    "r4AQF5mj",
    "nZ3E8WQz",
    "tNZqMcok",
    "TdLuRq7y",
    "uhBpdFWZ",
];

/// Components that the installed list folds into the single "Private Pack"
/// card instead of showing as their own entries.
///
/// Derived from `REMOVABLE_PACK_COMPONENTS` so a component added to the pack can
/// never keep appearing as a loose card: everything the pack installs and
/// removes is also everything it hides. `local-optifine` is excluded because it
/// is the record that becomes the pack card itself. The bundled core module is
/// part of the pack but is not removable, so it is listed separately.
pub(crate) fn is_folded_pack_component(project_id: &str) -> bool {
    project_id == "private-client-core"
        || (project_id != "local-optifine" && REMOVABLE_PACK_COMPONENTS.contains(&project_id))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn remove_mod(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<CommandResult> {
    if project_id == "private-pack" {
        let installed = crate::mods::list_installed(&state)?;
        let mut queued = false;
        for component in REMOVABLE_PACK_COMPONENTS {
            if installed.iter().any(|item| item.project_id == *component) {
                let result = crate::mods::remove(
                    &state,
                    RemoveModRequest {
                        project_id: (*component).to_owned(),
                    },
                )
                .await?;
                queued |= result.queued;
            }
        }
        return Ok(operation_result("Private Pack was removed", queued));
    }
    let result = crate::mods::remove(&state, RemoveModRequest { project_id }).await?;
    Ok(operation_result("The mod was removed", result.queued))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn update_mod(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<CommandResult> {
    let result = crate::mods::update(
        &state,
        UpdateModRequest {
            project_id,
            allow_beta: true,
        },
    )
    .await?;
    Ok(operation_result("The mod was updated", result.queued))
}

#[tauri::command]
pub fn list_installed_mods(state: State<'_, Arc<AppState>>) -> AppResult<Vec<InstalledMod>> {
    let mut mods = crate::mods::list_installed(&state)?;
    let pack_size = mods
        .iter()
        .filter(|item| {
            is_folded_pack_component(&item.project_id) || item.project_id == "local-optifine"
        })
        .map(|item| item.file_size)
        .sum();
    // `import_private_pack` only reports success after every component has been
    // registered. Use the locally registered OptiFine entry as the durable pack
    // anchor instead of hiding the whole card when an older installation uses a
    // different component identifier.
    let has_pack_anchor = mods.iter().any(|item| item.project_id == "local-optifine");
    mods.retain(|item| !is_folded_pack_component(&item.project_id));
    if !has_pack_anchor {
        mods.retain(|item| item.project_id != "local-optifine");
    } else if let Some(pack) = mods
        .iter_mut()
        .find(|item| item.project_id == "local-optifine")
    {
        pack.id = "private-pack".to_owned();
        pack.project_id = "private-pack".to_owned();
        pack.name = "Private Pack".to_owned();
        pack.author = "Private Client + external authors".to_owned();
        pack.description =
            "Managed Private Pack containing OptiFine, performance, PvP and visual components."
                .to_owned();
        pack.version = "1.0.0".to_owned();
        pack.installed_version = Some("1.0.0".to_owned());
        pack.file_size = pack_size;
        pack.provider = crate::contracts::ModSource::PrivateClient;
        pack.trust = crate::contracts::ModTrust::Verified;
        pack.required = false;
    }
    Ok(mods)
}

#[tauri::command]
pub fn list_pending_operations(
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<PendingOperation>> {
    Ok(crate::mods::list_pending(&state)?
        .iter()
        .map(PendingOperation::from)
        .collect())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn cancel_pending_operation(
    state: State<'_, Arc<AppState>>,
    operation_id: String,
) -> AppResult<CommandResult> {
    crate::mods::cancel_pending(&state, &operation_id).await?;
    Ok(CommandResult::completed(
        "The pending operation was cancelled",
    ))
}

#[tauri::command]
pub async fn apply_pending_operations(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    crate::mods::apply_pending(&state).await?;
    Ok(CommandResult::completed("Pending operations were applied"))
}

#[tauri::command]
pub async fn download_optifine(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    crate::optifine::download_and_import(&state).await?;
    Ok(CommandResult::completed(
        "Private Pack was downloaded, verified and installed automatically",
    ))
}

#[tauri::command]
pub async fn import_optifine(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    let app = state.app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("OptiFine JAR", &["jar"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| {
        AppError::new(
            AppErrorCode::Io,
            "The native OptiFine file picker could not be opened",
        )
        .details(error.to_string())
    })?;
    let Some(selected) = selected else {
        return Ok(CommandResult::completed("OptiFine import cancelled"));
    };
    let path = selected.into_path().map_err(|error| {
        AppError::new(
            AppErrorCode::InvalidInput,
            "The selected OptiFine location is not a local file",
        )
        .details(error.to_string())
    })?;
    let path = validate_picker_path(path)?;
    let result = crate::optifine::import(
        &state,
        crate::contracts::ImportOptifineRequest {
            source_path: path.to_string_lossy().into_owned(),
        },
    )
    .await?;
    Ok(operation_result(
        "Private Pack was fully imported and verified",
        result.queued,
    ))
}

fn operation_result(message: &str, queued: bool) -> CommandResult {
    if queued {
        CommandResult::queued(format!(
            "{message}; the change is waiting for the game to close"
        ))
    } else {
        CommandResult::completed(message)
    }
}

fn validate_picker_path(path: PathBuf) -> AppResult<PathBuf> {
    if !path.is_absolute()
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("jar"))
    {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "The native picker must return an absolute JAR path",
        ));
    }
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| AppError::io("Could not inspect the selected OptiFine JAR", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::new(
            AppErrorCode::SymlinkDetected,
            "The selected OptiFine JAR must be a regular local file",
        ));
    }
    Ok(path)
}

/// Resolves whether a newer signed launcher build is published. Safe to call on
/// demand; the automatic variant is gated by the `autoUpdateChecks` setting in
/// the frontend so the user keeps control of background network access.
#[tauri::command]
pub async fn check_for_update(state: State<'_, Arc<AppState>>) -> AppResult<UpdateStatus> {
    crate::updater::check(state.inner()).await
}

/// Installs the pending signed update and hands off to the installer.
#[tauri::command]
pub async fn install_update(state: State<'_, Arc<AppState>>) -> AppResult<CommandResult> {
    crate::updater::install(state.inner()).await?;
    Ok(CommandResult::completed(
        "The update was downloaded and verified",
    ))
}

#[cfg(test)]
mod tests {
    use super::validate_picker_path;
    use std::path::PathBuf;

    #[test]
    fn picker_boundary_rejects_frontend_style_relative_paths() {
        assert!(validate_picker_path(PathBuf::from("OptiFine_1.8.9.jar")).is_err());
        assert!(validate_picker_path(PathBuf::from(r"C:\Temp\OptiFine_1.8.9.txt")).is_err());
    }
}
