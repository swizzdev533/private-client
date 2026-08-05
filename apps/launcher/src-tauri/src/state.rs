use crate::config::ConfigStore;
use crate::contracts::{LaunchPhase, LaunchState, EVENT_LAUNCH_STATE};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::{atomic_write_json, read_json};
use crate::logging::LocalLogger;
use crate::network::SecureHttpClient;
use crate::paths::PathLayout;
use fs2::FileExt;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::{File, OpenOptions};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use sysinfo::{Pid, System};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameProcessRecord {
    schema_version: u32,
    pub pid: u32,
    pub start_time: u64,
    executable_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveProcessIdentity {
    start_time: u64,
    executable_name: String,
}

pub struct AppState {
    pub app: AppHandle,
    pub paths: PathLayout,
    pub config: ConfigStore,
    pub network: SecureHttpClient,
    pub logger: LocalLogger,
    pub launch: Arc<RwLock<LaunchState>>,
    pub cancel_launch: Arc<AtomicBool>,
    pub stop_requested: Arc<AtomicBool>,
    game_identity: Arc<Mutex<Option<GameProcessRecord>>>,
    pub game_lock: Arc<Mutex<Option<File>>>,
    pub operation_lock: Arc<tokio::sync::Mutex<()>>,
    pub profile_watcher_stop: Arc<AtomicBool>,
    pub profile_watcher_running: Arc<AtomicBool>,
    _launcher_lock: File,
}

impl AppState {
    pub fn initialize(app: AppHandle) -> AppResult<Self> {
        let paths = PathLayout::discover()?;
        paths.ensure()?;
        let recovered_game = reconcile_game_process(&paths)?;
        let launcher_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&paths.launcher_lock)
            .map_err(|error| AppError::io("Could not open the launcher instance lock", error))?;
        FileExt::try_lock_exclusive(&launcher_lock).map_err(|error| {
            AppError::new(
                AppErrorCode::SingleInstanceViolation,
                "Private Client is already running",
            )
            .details(error.to_string())
        })?;
        let logger = LocalLogger::new(&paths)?;
        logger.info(
            "startup",
            "Private Client backend initialized; telemetry disabled",
        );
        let config = ConfigStore::new(paths.clone());
        let network = SecureHttpClient::new()?;
        let launch = if let Some(record) = &recovered_game {
            LaunchState {
                state: LaunchPhase::Running,
                message: "Minecraft is still running".to_owned(),
                progress: Some(100.0),
                can_cancel: false,
                pid: Some(record.pid),
                ..LaunchState::default()
            }
        } else {
            LaunchState::default()
        };
        Ok(Self {
            app,
            paths,
            config,
            network,
            logger,
            launch: Arc::new(RwLock::new(launch)),
            cancel_launch: Arc::new(AtomicBool::new(false)),
            stop_requested: Arc::new(AtomicBool::new(false)),
            game_identity: Arc::new(Mutex::new(recovered_game)),
            game_lock: Arc::new(Mutex::new(None)),
            operation_lock: Arc::new(tokio::sync::Mutex::new(())),
            profile_watcher_stop: Arc::new(AtomicBool::new(false)),
            profile_watcher_running: Arc::new(AtomicBool::new(false)),
            _launcher_lock: launcher_lock,
        })
    }

    pub fn launch_state(&self) -> LaunchState {
        self.launch.read().clone()
    }

    pub fn set_launch_state(&self, next: LaunchState) {
        *self.launch.write() = next.clone();
        let _ = self.app.emit(EVENT_LAUNCH_STATE, next);
    }

    pub fn request_cancel(&self) {
        self.cancel_launch.store(true, Ordering::SeqCst);
    }

    pub fn clear_cancel(&self) {
        self.cancel_launch.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_launch.load(Ordering::SeqCst)
    }

    pub fn is_game_running(&self) -> bool {
        self.live_game_process().is_some()
    }

    pub fn live_game_pid(&self) -> Option<u32> {
        self.live_game_process().map(|record| record.pid)
    }

    pub fn live_game_process(&self) -> Option<GameProcessRecord> {
        let mut identity = self.game_identity.lock();
        let record = identity.clone()?;
        if record_matches_live(&record, query_live_process(record.pid).as_ref()) {
            Some(record)
        } else {
            self.clear_game_process_locked(&mut identity, &record);
            None
        }
    }

    /// Revalidates the entire persisted identity, not just the PID. Call this
    /// immediately before any OS operation that can affect a process.
    pub fn is_game_process_running(&self, expected: &GameProcessRecord) -> bool {
        let mut identity = self.game_identity.lock();
        match owned_process_status(
            identity.as_ref(),
            expected,
            query_live_process(expected.pid).as_ref(),
        ) {
            OwnedProcessStatus::Live => true,
            OwnedProcessStatus::Gone => {
                self.clear_game_process_locked(&mut identity, expected);
                false
            }
            OwnedProcessStatus::Superseded => false,
        }
    }

    pub async fn register_game_process(
        &self,
        pid: u32,
        executable: &str,
    ) -> AppResult<GameProcessRecord> {
        let expected_name = std::path::Path::new(executable)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("java.exe")
            .to_ascii_lowercase();
        let mut live = None;
        for _ in 0..20 {
            live = query_live_process(pid);
            if live.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        let live = live.ok_or_else(|| {
            AppError::new(
                AppErrorCode::LaunchFailed,
                "The Java process identity could not be verified",
            )
        })?;
        if live.executable_name != expected_name {
            return Err(AppError::new(
                AppErrorCode::LaunchFailed,
                "The spawned process does not match the selected Java runtime",
            ));
        }
        let record = GameProcessRecord {
            schema_version: 1,
            pid,
            start_time: live.start_time,
            executable_name: live.executable_name,
        };
        // Keep the persisted and in-memory identities in one critical section.
        // Stop/finalization transitions use this same mutex as their lifecycle
        // boundary, so they cannot observe a half-registered process.
        let mut identity = self.game_identity.lock();
        atomic_write_json(&self.paths.game_process, &record)?;
        *identity = Some(record.clone());
        Ok(record)
    }

    pub fn begin_game_stop(&self, expected: &GameProcessRecord, next: LaunchState) -> bool {
        self.set_launch_state_if_game_process_live_from(
            expected,
            GameLaunchTransition::BeginStop,
            next,
            false,
        )
    }

    /// A watcher cannot clear the process between validation and this state
    /// write, so a failed stop cannot publish RUNNING after EXITED.
    pub fn restore_game_running_after_stop_failure(
        &self,
        expected: &GameProcessRecord,
        next: LaunchState,
    ) -> bool {
        self.set_launch_state_if_game_process_live_from(
            expected,
            GameLaunchTransition::RestoreRunning,
            next,
            true,
        )
    }

    fn set_launch_state_if_game_process_live_from(
        &self,
        expected: &GameProcessRecord,
        transition: GameLaunchTransition,
        next: LaunchState,
        reset_stop_request: bool,
    ) -> bool {
        let mut identity = self.game_identity.lock();
        match owned_process_status(
            identity.as_ref(),
            expected,
            query_live_process(expected.pid).as_ref(),
        ) {
            OwnedProcessStatus::Live => {
                let mut launch = self.launch.write();
                if !launch_transition_allowed(&launch.state, launch.pid, expected.pid, transition) {
                    return false;
                }
                if reset_stop_request {
                    self.stop_requested.store(false, Ordering::SeqCst);
                }
                *launch = next.clone();
                drop(launch);
                let _ = self.app.emit(EVENT_LAUNCH_STATE, next);
                true
            }
            OwnedProcessStatus::Gone => {
                self.clear_game_process_locked(&mut identity, expected);
                false
            }
            OwnedProcessStatus::Superseded => false,
        }
    }

    /// Publishes a terminal state only when the expected process is gone and no
    /// newer process record has replaced it. Existing terminal watcher output is
    /// preserved and also counts as successful finalization.
    pub fn set_launch_state_if_game_process_stopped(
        &self,
        expected: &GameProcessRecord,
        next: LaunchState,
    ) -> bool {
        let mut identity = self.game_identity.lock();
        match owned_process_status(
            identity.as_ref(),
            expected,
            query_live_process(expected.pid).as_ref(),
        ) {
            OwnedProcessStatus::Live => return false,
            OwnedProcessStatus::Gone => {
                self.clear_game_process_locked(&mut identity, expected);
            }
            OwnedProcessStatus::Superseded => return false,
        }

        let mut launch = self.launch.write();
        if launch_is_terminal(&launch.state, launch.pid) {
            return true;
        }
        if !launch_transition_allowed(
            &launch.state,
            launch.pid,
            expected.pid,
            GameLaunchTransition::FinishStop,
        ) {
            return false;
        }
        *launch = next.clone();
        drop(launch);
        let _ = self.app.emit(EVENT_LAUNCH_STATE, next);
        true
    }

    fn clear_game_process_locked(
        &self,
        identity: &mut Option<GameProcessRecord>,
        expected: &GameProcessRecord,
    ) {
        if identity.as_ref() != Some(expected) {
            return;
        }
        let remove_record = match read_json::<GameProcessRecord>(&self.paths.game_process) {
            Ok(record) => record == *expected,
            Err(_) => true,
        };
        if remove_record {
            let _ = fs::remove_file(&self.paths.game_process);
        }
        *identity = None;
    }

    pub fn stop_background_tasks(&self) {
        self.profile_watcher_stop.store(true, Ordering::SeqCst);
    }
}

fn reconcile_game_process(paths: &PathLayout) -> AppResult<Option<GameProcessRecord>> {
    if !paths.game_process.exists() {
        return Ok(None);
    }
    let record = match read_json::<GameProcessRecord>(&paths.game_process) {
        Ok(record) if record.schema_version == 1 => record,
        Ok(_) | Err(_) => {
            fs::remove_file(&paths.game_process)
                .map_err(|error| AppError::io("Could not remove an invalid game record", error))?;
            return Ok(None);
        }
    };
    if record_matches_live(&record, query_live_process(record.pid).as_ref()) {
        Ok(Some(record))
    } else {
        fs::remove_file(&paths.game_process)
            .map_err(|error| AppError::io("Could not remove a stale game record", error))?;
        Ok(None)
    }
}

fn query_live_process(pid: u32) -> Option<LiveProcessIdentity> {
    let system = System::new_all();
    let process = system.process(Pid::from_u32(pid))?;
    let executable_name = process
        .exe()
        .and_then(std::path::Path::file_name)
        .unwrap_or_else(|| process.name())
        .to_string_lossy()
        .to_ascii_lowercase();
    Some(LiveProcessIdentity {
        start_time: process.start_time(),
        executable_name,
    })
}

fn record_matches_live(record: &GameProcessRecord, live: Option<&LiveProcessIdentity>) -> bool {
    live.is_some_and(|live| {
        record.start_time != 0
            && record.start_time == live.start_time
            && record
                .executable_name
                .eq_ignore_ascii_case(&live.executable_name)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnedProcessStatus {
    Live,
    Gone,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameLaunchTransition {
    BeginStop,
    RestoreRunning,
    FinishStop,
}

fn launch_transition_allowed(
    phase: &LaunchPhase,
    current_pid: Option<u32>,
    expected_pid: u32,
    transition: GameLaunchTransition,
) -> bool {
    if current_pid != Some(expected_pid) {
        return false;
    }
    match transition {
        GameLaunchTransition::BeginStop => phase == &LaunchPhase::Running,
        GameLaunchTransition::RestoreRunning => phase == &LaunchPhase::Stopping,
        GameLaunchTransition::FinishStop => {
            matches!(phase, LaunchPhase::Running | LaunchPhase::Stopping)
        }
    }
}

fn launch_is_terminal(phase: &LaunchPhase, pid: Option<u32>) -> bool {
    pid.is_none() && matches!(phase, LaunchPhase::Exited | LaunchPhase::Failed)
}

fn owned_process_status(
    current: Option<&GameProcessRecord>,
    expected: &GameProcessRecord,
    live: Option<&LiveProcessIdentity>,
) -> OwnedProcessStatus {
    match current {
        Some(current) if current == expected => {
            if record_matches_live(expected, live) {
                OwnedProcessStatus::Live
            } else {
                OwnedProcessStatus::Gone
            }
        }
        Some(_) => OwnedProcessStatus::Superseded,
        None => OwnedProcessStatus::Gone,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        launch_is_terminal, launch_transition_allowed, owned_process_status, record_matches_live,
        GameLaunchTransition, GameProcessRecord, LaunchPhase, LiveProcessIdentity,
        OwnedProcessStatus,
    };

    fn record() -> GameProcessRecord {
        GameProcessRecord {
            schema_version: 1,
            pid: 42,
            start_time: 100,
            executable_name: "javaw.exe".to_owned(),
        }
    }

    #[test]
    fn persistent_process_identity_rejects_stale_or_reused_pids() {
        assert!(record_matches_live(
            &record(),
            Some(&LiveProcessIdentity {
                start_time: 100,
                executable_name: "javaw.exe".to_owned(),
            })
        ));
        assert!(!record_matches_live(
            &record(),
            Some(&LiveProcessIdentity {
                start_time: 101,
                executable_name: "javaw.exe".to_owned(),
            })
        ));
        assert!(!record_matches_live(
            &record(),
            Some(&LiveProcessIdentity {
                start_time: 100,
                executable_name: "java.exe".to_owned(),
            })
        ));
        assert!(!record_matches_live(&record(), None));
    }

    #[test]
    fn lifecycle_transition_requires_the_exact_registered_process() {
        let expected = record();
        let matching_live = LiveProcessIdentity {
            start_time: 100,
            executable_name: "javaw.exe".to_owned(),
        };
        assert_eq!(
            owned_process_status(Some(&expected), &expected, Some(&matching_live)),
            OwnedProcessStatus::Live
        );

        let mut replacement = record();
        replacement.start_time = 101;
        assert_eq!(
            owned_process_status(Some(&replacement), &expected, Some(&matching_live)),
            OwnedProcessStatus::Superseded
        );
        assert_eq!(
            owned_process_status(Some(&expected), &expected, None),
            OwnedProcessStatus::Gone
        );
    }

    #[test]
    fn recovered_stopping_state_can_finish_but_cannot_restore_a_different_process() {
        assert!(launch_transition_allowed(
            &LaunchPhase::Stopping,
            Some(42),
            42,
            GameLaunchTransition::FinishStop
        ));
        assert!(launch_transition_allowed(
            &LaunchPhase::Running,
            Some(42),
            42,
            GameLaunchTransition::FinishStop
        ));
        assert!(!launch_transition_allowed(
            &LaunchPhase::Stopping,
            Some(43),
            42,
            GameLaunchTransition::RestoreRunning
        ));
        assert!(!launch_transition_allowed(
            &LaunchPhase::Exited,
            None,
            42,
            GameLaunchTransition::FinishStop
        ));
        assert!(launch_is_terminal(&LaunchPhase::Exited, None));
        assert!(launch_is_terminal(&LaunchPhase::Failed, None));
    }
}
