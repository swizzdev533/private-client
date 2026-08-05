pub mod commands;
pub mod config;
pub mod contracts;
pub mod diagnostics;
pub mod error;
pub mod fs_secure;
pub mod java;
pub mod launcher;
pub mod logging;
pub mod minecraft;
pub mod modrinth;
pub mod mods;
pub mod network;
pub mod optifine;
pub mod paths;
pub mod profile;
pub mod runtime_java;
pub mod state;
pub mod updater;

#[cfg(not(test))]
use crate::error::AppErrorCode;
#[cfg(not(test))]
use crate::state::AppState;
#[cfg(not(test))]
use std::sync::Arc;
#[cfg(not(test))]
use tauri::{Manager, WindowEvent};
#[cfg(not(test))]
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

#[cfg(not(test))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = match AppState::initialize(app.handle().clone()) {
                Ok(s) => Arc::new(s),
                Err(err) => {
                    let is_single_instance =
                        matches!(err.code, AppErrorCode::SingleInstanceViolation);
                    let title = if is_single_instance {
                        "Private Client Jest Już Uruchomiony"
                    } else {
                        "Błąd Uruchamiania Private Client"
                    };
                    let message = if is_single_instance {
                        "Aplikacja Private Client jest już uruchomiona w tle.\n\nSprawdź pasek zadań lub Menedżer Zadań."
                    } else {
                        &format!("Nie udało się zainicjalizować aplikacji Private Client:\n\n{}", err.message)
                    };

                    app.dialog()
                        .message(message)
                        .title(title)
                        .kind(MessageDialogKind::Error)
                        .blocking_show();

                    return Err(Box::<dyn std::error::Error>::from(err));
                }
            };
            crate::profile::start_watcher(
                app.handle().clone(),
                state.paths.clone(),
                state.logger.clone(),
                state.network.clone(),
                Arc::clone(&state.profile_watcher_stop),
                Arc::clone(&state.profile_watcher_running),
            );
            if let Some(process) = state.live_game_process() {
                let monitor = Arc::clone(&state);
                tauri::async_runtime::spawn(async move {
                    while monitor.is_game_process_running(&process) {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                    let _operation = monitor.operation_lock.lock().await;
                    let mut launch = monitor.launch_state();
                    if matches!(
                        &launch.state,
                        crate::contracts::LaunchPhase::Running
                            | crate::contracts::LaunchPhase::Stopping
                    ) {
                        launch.state = crate::contracts::LaunchPhase::Exited;
                        launch.message = "Minecraft closed".to_owned();
                        launch.progress = None;
                        launch.can_cancel = false;
                        launch.error_id = None;
                        launch.pid = None;
                        launch.finished_at = Some(chrono::Utc::now().to_rfc3339());
                        launch.exit_code = None;
                        launch.crash_kind = Some(crate::contracts::CrashKind::CleanExit);
                        let _ =
                            monitor.set_launch_state_if_game_process_stopped(&process, launch);
                    }
                });
            }
            app.manage(Arc::clone(&state));
            let state_for_mods = Arc::clone(&state);
            tauri::async_runtime::spawn(async move {
                let _ = crate::mods::ensure_required_mods(&state_for_mods).await;
            });
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_launcher_snapshot,
            commands::launch_game,
            commands::cancel_launch,
            commands::stop_game,
            commands::focus_game_window,
            commands::save_launcher_settings,
            commands::open_logs_directory,
            commands::export_logs,
            commands::search_modrinth,
            commands::get_mod_install_plan,
            commands::install_mod,
            commands::remove_mod,
            commands::update_mod,
            commands::list_installed_mods,
            commands::list_pending_operations,
            commands::cancel_pending_operation,
            commands::apply_pending_operations,
            commands::download_optifine,
            commands::import_optifine,
            commands::check_for_update,
            commands::install_update,
        ])
        .on_window_event(|window, event| {
            if matches!(event, WindowEvent::Destroyed) {
                if let Some(state) = window.try_state::<Arc<AppState>>() {
                    state.stop_background_tasks();
                }
            }
        });

    if let Err(error) = builder.run(tauri::generate_context!()) {
        eprintln!("Private Client failed to start: {error}");
    }
}
