use crate::config::{validate_memory, validate_memory_for_system};
use crate::contracts::{CommandResult, CrashKind, LaunchPhase, LaunchRequest, LaunchState};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::read_json;
use crate::java;
use crate::logging::{redact, LocalLogger};
use crate::minecraft::{
    self, library_applies, maven_path, MojangLibrary, VersionMetadata, FORGE_VERSION_ID,
    MINECRAFT_VERSION,
};
use crate::state::{AppState, GameProcessRecord};
use chrono::Utc;
use fs2::FileExt;
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::{BTreeSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

const OFFLINE_USERNAME: &str = "LoginInGame";
const OFFLINE_UUID: &str = "00000000-0000-0000-0000-000000000000";
const TASKKILL_TIMEOUT: Duration = Duration::from_secs(15);
const STOP_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Deserialize)]
struct ForgeLaunchMetadata {
    id: String,
    #[serde(rename = "mainClass")]
    main_class: String,
    #[serde(default)]
    libraries: Vec<ForgeLaunchLibrary>,
}

#[derive(Debug, Deserialize)]
struct ForgeLaunchLibrary {
    name: String,
    clientreq: Option<bool>,
}

pub async fn prepare_instance(state: &AppState) -> AppResult<crate::contracts::InstanceSnapshot> {
    let _guard = state.operation_lock.lock().await;
    if state.is_game_running() {
        return Err(AppError::new(
            AppErrorCode::OperationBlockedWhileRunning,
            "The instance cannot be repaired while the game is active",
        ));
    }
    state.clear_cancel();
    let mut launch = state.launch_state();
    launch.state = LaunchPhase::InstallingGameFiles;
    launch.progress = Some(3.0);
    launch.can_cancel = true;
    launch.message = "Preparing isolated Minecraft 1.8.9 instance".to_owned();
    launch.error_id = None;
    launch.log_path = None;
    launch.exit_code = None;
    launch.crash_kind = None;
    state.set_launch_state(launch);
    let result = minecraft::install_or_repair(
        &state.app,
        &state.paths,
        &state.network,
        &state.logger,
        Arc::clone(&state.cancel_launch),
    )
    .await;
    let result = match result {
        Ok(snapshot) => crate::mods::ensure_required_mods(state)
            .await
            .map(|_| snapshot),
        Err(error) => Err(error),
    };
    match result {
        Ok(snapshot) => {
            let mut next = state.launch_state();
            next.state = LaunchPhase::Idle;
            next.progress = None;
            next.can_cancel = false;
            next.message = "Instance ready".to_owned();
            state.set_launch_state(next);
            Ok(snapshot)
        }
        Err(error) => {
            let mut next = state.launch_state();
            next.state = LaunchPhase::Failed;
            next.progress = None;
            next.can_cancel = false;
            next.message = error.message.to_string();
            next.error_id = Some(format!("{:?}", error.code));
            next.log_path = Some(state.logger.path().to_string_lossy().into_owned());
            state.set_launch_state(next);
            Err(error.with_log(state.logger.path()))
        }
    }
}

pub async fn launch(state: Arc<AppState>, request: LaunchRequest) -> AppResult<LaunchState> {
    validate_memory(request.minimum_memory_mb, request.maximum_memory_mb)?;
    validate_memory_for_system(request.maximum_memory_mb)?;
    if state.is_game_running() {
        return Err(AppError::new(
            AppErrorCode::GameAlreadyRunning,
            "Minecraft is already running",
        ));
    }
    let _guard = state.operation_lock.lock().await;
    if state.is_game_running()
        || !matches!(
            state.launch_state().state,
            LaunchPhase::Idle | LaunchPhase::Exited | LaunchPhase::Failed
        )
    {
        return Err(AppError::new(
            AppErrorCode::GameAlreadyRunning,
            "A launch operation is already active",
        ));
    }
    state.clear_cancel();
    state.stop_requested.store(false, Ordering::SeqCst);
    set_phase(
        &state,
        LaunchPhase::Validating,
        0.01,
        "Validating Java and local instance",
    );
    let java = match java::require_java8(&state.config, &state.paths, None).await {
        Ok(candidate) => candidate,
        Err(_) => {
            set_phase(
                &state,
                LaunchPhase::CheckingRuntime,
                0.02,
                "Downloading Java 8 runtime",
            );
            java::ensure_java8(&state).await?
        }
    };
    set_phase(
        &state,
        LaunchPhase::VerifyingGameFiles,
        0.03,
        "Verifying the isolated Minecraft instance",
    );
    minecraft::install_or_repair(
        &state.app,
        &state.paths,
        &state.network,
        &state.logger,
        Arc::clone(&state.cancel_launch),
    )
    .await?;
    if state.is_cancelled() {
        set_phase(&state, LaunchPhase::Idle, 0.0, "Launch cancelled");
        return Err(AppError::new(
            AppErrorCode::OperationBlockedWhileRunning,
            "The launch was cancelled",
        ));
    }
    set_phase(
        &state,
        LaunchPhase::CheckingRequiredMods,
        0.86,
        "Verifying pinned required mods",
    );
    crate::mods::ensure_required_mods(&state).await?;
    validate_installed_mods(&state)?;
    let lock = acquire_game_lock(&state)?;
    let specification = build_launch_specification(&state, &java.executable, &request)?;
    set_phase(
        &state,
        LaunchPhase::Launching,
        0.94,
        "Starting Minecraft Forge",
    );
    let mut command = specification.command;
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            FileExt::unlock(&lock).ok();
            return Err(AppError::new(
                AppErrorCode::LaunchFailed,
                "The Java process could not be started",
            )
            .details(error.to_string())
            .with_log(state.logger.path()));
        }
    };
    let pid = child.id().ok_or_else(|| {
        AppError::new(
            AppErrorCode::LaunchFailed,
            "The Java process did not expose a process ID",
        )
    })?;
    let process_record = match state.register_game_process(pid, &java.executable).await {
        Ok(record) => record,
        Err(error) => {
            let _ = child.kill().await;
            FileExt::unlock(&lock).ok();
            return Err(error.with_log(state.logger.path()));
        }
    };
    *state.game_lock.lock() = Some(lock);
    let tail = Arc::new(Mutex::new(VecDeque::<String>::with_capacity(200)));
    let stdout_task = child.stdout.take().map(|stdout| {
        let logger = state.logger.clone();
        let tail = Arc::clone(&tail);
        tokio::spawn(drain_output(stdout, logger, "minecraft.stdout", tail))
    });
    let stderr_task = child.stderr.take().map(|stderr| {
        let logger = state.logger.clone();
        let tail = Arc::clone(&tail);
        tokio::spawn(drain_output(stderr, logger, "minecraft.stderr", tail))
    });
    let running = LaunchState {
        state: LaunchPhase::Running,
        message: "Minecraft 1.8.9 Forge is running".to_owned(),
        progress: Some(100.0),
        can_cancel: false,
        error_id: None,
        log_path: Some(state.logger.path().to_string_lossy().into_owned()),
        pid: Some(pid),
        started_at: Some(Utc::now().to_rfc3339()),
        finished_at: None,
        exit_code: None,
        crash_kind: None,
    };
    state.set_launch_state(running.clone());
    state.logger.info(
        "launcher",
        format!("Started isolated Minecraft process pid={pid}; authentication remains in-game"),
    );
    let watcher_state = Arc::clone(&state);
    tauri::async_runtime::spawn(async move {
        let status = child.wait().await;
        if let Some(task) = stdout_task {
            let _ = task.await;
        }
        if let Some(task) = stderr_task {
            let _ = task.await;
        }
        if let Some(lock) = watcher_state.game_lock.lock().take() {
            let _ = FileExt::unlock(&lock);
        }
        let exit_code = status
            .as_ref()
            .ok()
            .and_then(std::process::ExitStatus::code);
        let lines: Vec<String> = tail.lock().iter().cloned().collect();
        let crash = classify_exit(
            exit_code,
            &lines,
            watcher_state.stop_requested.load(Ordering::SeqCst),
        );
        let clean = matches!(crash, CrashKind::CleanExit | CrashKind::UserTerminated);
        let next = LaunchState {
            state: if clean {
                LaunchPhase::Exited
            } else {
                LaunchPhase::Failed
            },
            message: if clean {
                "Minecraft closed".to_owned()
            } else {
                "Minecraft exited unexpectedly; local logs are available".to_owned()
            },
            progress: None,
            can_cancel: false,
            error_id: if clean {
                None
            } else {
                Some("GameCrashed".to_owned())
            },
            log_path: Some(watcher_state.logger.path().to_string_lossy().into_owned()),
            pid: None,
            started_at: watcher_state.launch_state().started_at,
            finished_at: Some(Utc::now().to_rfc3339()),
            exit_code,
            crash_kind: Some(crash),
        };
        if !watcher_state.set_launch_state_if_game_process_stopped(&process_record, next) {
            watcher_state.logger.warn(
                "launcher",
                "Ignored a stale Minecraft watcher transition for a different process",
            );
        }
        if let Err(error) = status {
            watcher_state
                .logger
                .error("launcher", format!("Failed to wait for Minecraft: {error}"));
        }
        if let Err(error) = crate::mods::apply_pending(&watcher_state).await {
            watcher_state.logger.warn(
                "mods.queue",
                format!("Pending operations were not applied: {error}"),
            );
        }
    });
    Ok(running)
}

pub async fn stop(state: &AppState) -> AppResult<CommandResult> {
    let _operation = state.operation_lock.lock().await;
    let process = state
        .live_game_process()
        .ok_or_else(|| AppError::new(AppErrorCode::GameNotRunning, "Minecraft is not running"))?;
    state.stop_requested.store(true, Ordering::SeqCst);
    let mut stopping = state.launch_state();
    stopping.state = LaunchPhase::Stopping;
    stopping.message = "Stopping Minecraft".to_owned();
    stopping.progress = Some(100.0);
    stopping.can_cancel = false;
    stopping.error_id = None;
    if !state.begin_game_stop(&process, stopping) {
        if complete_stop_if_stopped(state, &process) {
            return Ok(CommandResult::completed("Minecraft stopped"));
        }
        state.stop_requested.store(false, Ordering::SeqCst);
        return Err(AppError::new(
            AppErrorCode::LaunchFailed,
            "Minecraft changed state before it could be stopped",
        ));
    }

    let graceful = run_verified_taskkill(state, &process, false).await;
    if taskkill_attempt_succeeded(&graceful)
        && wait_for_game_exit(state, &process).await
        && complete_stop_if_stopped(state, &process)
    {
        return Ok(CommandResult::completed("Minecraft stopped"));
    }

    if matches!(
        stop_decision(false, state.is_game_process_running(&process)),
        StopDecision::Complete
    ) && complete_stop_if_stopped(state, &process)
    {
        return Ok(CommandResult::completed("Minecraft stopped"));
    }

    state.logger.warn(
        "launcher",
        format!(
            "Graceful Minecraft stop did not end the verified process; using forced tree termination ({})",
            redact(&taskkill_attempt_details(&graceful))
        ),
    );
    let forced = run_verified_taskkill(state, &process, true).await;
    if taskkill_attempt_succeeded(&forced)
        && wait_for_game_exit(state, &process).await
        && complete_stop_if_stopped(state, &process)
    {
        return Ok(CommandResult::completed("Minecraft stopped"));
    }

    match stop_decision(true, state.is_game_process_running(&process)) {
        StopDecision::Complete if complete_stop_if_stopped(state, &process) => {
            Ok(CommandResult::completed("Minecraft stopped"))
        }
        StopDecision::Retry => {
            if !restore_running_after_stop_failure(state, &process) {
                if complete_stop_if_stopped(state, &process) {
                    return Ok(CommandResult::completed("Minecraft stopped"));
                }
                return Err(AppError::new(
                    AppErrorCode::LaunchFailed,
                    "Minecraft process state changed while stopping",
                ));
            }
            let details = format!(
                "graceful: {}\nforced: {}",
                taskkill_attempt_details(&graceful),
                taskkill_attempt_details(&forced)
            );
            Err(AppError::new(
                AppErrorCode::LaunchFailed,
                "Windows could not stop the Minecraft process",
            )
            .details(redact(&details)))
        }
        StopDecision::Complete => Err(AppError::new(
            AppErrorCode::LaunchFailed,
            "Minecraft process identity changed while stopping",
        )),
        StopDecision::Force => unreachable!("a forced attempt cannot request another fallback"),
    }
}

async fn run_verified_taskkill(
    state: &AppState,
    expected: &GameProcessRecord,
    force: bool,
) -> AppResult<Option<std::process::Output>> {
    // taskkill accepts only a PID, while the launcher also supports processes
    // recovered after restart (where no owned Child handle survives). The target
    // therefore comes only from backend state and its PID, start time, and image
    // name are revalidated here immediately before every graceful/forced attempt.
    if !state.is_game_process_running(expected) {
        return Ok(None);
    }
    run_taskkill(expected.pid, force).await.map(Some)
}

async fn run_taskkill(pid: u32, force: bool) -> AppResult<std::process::Output> {
    let mut command = Command::new("taskkill.exe");
    command
        .args(taskkill_arguments(pid, force))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    hide_console(&mut command);
    timeout(TASKKILL_TIMEOUT, command.output())
        .await
        .map_err(|_| {
            AppError::new(
                AppErrorCode::LaunchFailed,
                "Timed out while stopping Minecraft",
            )
        })?
        .map_err(|error| AppError::io("Could not request Minecraft shutdown", error))
}

async fn wait_for_game_exit(state: &AppState, expected: &GameProcessRecord) -> bool {
    if !state.is_game_process_running(expected) {
        return true;
    }
    timeout(STOP_CONFIRMATION_TIMEOUT, async {
        loop {
            tokio::time::sleep(STOP_POLL_INTERVAL).await;
            if !state.is_game_process_running(expected) {
                return;
            }
        }
    })
    .await
    .is_ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopDecision {
    Complete,
    Force,
    Retry,
}

fn stop_decision(forced_attempted: bool, process_running: bool) -> StopDecision {
    if !process_running {
        StopDecision::Complete
    } else if forced_attempted {
        StopDecision::Retry
    } else {
        StopDecision::Force
    }
}

fn complete_stop_if_stopped(state: &AppState, process: &GameProcessRecord) -> bool {
    let mut launch = state.launch_state();
    launch.state = LaunchPhase::Exited;
    launch.message = "Minecraft closed".to_owned();
    launch.progress = None;
    launch.can_cancel = false;
    launch.error_id = None;
    launch.pid = None;
    launch.finished_at = Some(Utc::now().to_rfc3339());
    launch.exit_code = None;
    launch.crash_kind = Some(CrashKind::UserTerminated);
    state.set_launch_state_if_game_process_stopped(process, launch)
}

fn restore_running_after_stop_failure(state: &AppState, process: &GameProcessRecord) -> bool {
    let mut launch = state.launch_state();
    launch.state = LaunchPhase::Running;
    launch.message = "Minecraft is still running; stopping failed".to_owned();
    launch.progress = Some(100.0);
    launch.can_cancel = false;
    launch.pid = Some(process.pid);
    state.restore_game_running_after_stop_failure(process, launch)
}

fn taskkill_attempt_succeeded(result: &AppResult<Option<std::process::Output>>) -> bool {
    match result {
        Ok(None) => true,
        Ok(Some(output)) => output.status.success(),
        Err(_) => false,
    }
}

fn taskkill_attempt_details(result: &AppResult<Option<std::process::Output>>) -> String {
    match result {
        Ok(Some(output)) => format!(
            "exit_code={:?}, stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ),
        Ok(None) => "skipped because the verified process was no longer running".to_owned(),
        Err(error) => error.to_string(),
    }
}

fn taskkill_arguments(pid: u32, force: bool) -> Vec<String> {
    let mut arguments = vec!["/PID".to_owned(), pid.to_string()];
    if force {
        arguments.push("/T".to_owned());
        arguments.push("/F".to_owned());
    }
    arguments
}

pub fn cancel(state: &AppState) -> CommandResult {
    if state.is_game_running() {
        return CommandResult::completed("Minecraft is already running; use stop_game to close it");
    }
    state.request_cancel();
    CommandResult::completed("Launch cancellation requested")
}

pub async fn focus_game(state: &AppState) -> AppResult<CommandResult> {
    let pid = state
        .live_game_pid()
        .ok_or_else(|| AppError::new(AppErrorCode::GameNotRunning, "Minecraft is not running"))?;
    let script = "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.Interaction]::AppActivate([int]$env:PRIVATE_CLIENT_GAME_PID)";
    let mut command = Command::new("powershell.exe");
    command
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(script)
        .env("PRIVATE_CLIENT_GAME_PID", pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_console(&mut command);
    let status = timeout(Duration::from_secs(5), command.status())
        .await
        .map_err(|_| {
            AppError::new(
                AppErrorCode::LaunchFailed,
                "Timed out while focusing Minecraft",
            )
        })?
        .map_err(|error| AppError::io("Could not focus the Minecraft window", error))?;
    Ok(CommandResult::completed(if status.success() {
        "Minecraft window focused".to_owned()
    } else {
        "Minecraft window is not ready to be focused".to_owned()
    }))
}

fn acquire_game_lock(state: &AppState) -> AppResult<File> {
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&state.paths.game_lock)
        .map_err(|error| AppError::io("Could not open the game process lock", error))?;
    FileExt::try_lock_exclusive(&lock).map_err(|error| {
        AppError::new(
            AppErrorCode::GameAlreadyRunning,
            "Another Minecraft process owns the isolated instance",
        )
        .details(error.to_string())
    })?;
    Ok(lock)
}

struct LaunchSpecification {
    command: Command,
}

fn build_launch_specification(
    state: &AppState,
    java_executable: &str,
    request: &LaunchRequest,
) -> AppResult<LaunchSpecification> {
    let vanilla_path = state
        .paths
        .versions
        .join(MINECRAFT_VERSION)
        .join(format!("{MINECRAFT_VERSION}.json"));
    let forge_path = state
        .paths
        .versions
        .join(FORGE_VERSION_ID)
        .join(format!("{FORGE_VERSION_ID}.json"));
    let vanilla: VersionMetadata = read_json(&vanilla_path)?;
    let forge: ForgeLaunchMetadata = read_json(&forge_path)?;
    if forge.id != FORGE_VERSION_ID || forge.main_class != "net.minecraft.launchwrapper.Launch" {
        return Err(AppError::new(
            AppErrorCode::InstanceCorrupted,
            "The Forge launch metadata is invalid",
        ));
    }
    let mut classpath = Vec::new();
    let mut seen = BTreeSet::new();
    for library in vanilla
        .libraries
        .iter()
        .filter(|library| library_applies(library))
    {
        add_mojang_library(&state.paths.libraries, library, &mut classpath, &mut seen)?;
    }
    for library in forge
        .libraries
        .into_iter()
        .filter(|library| library.clientreq.unwrap_or(true))
    {
        let path = state.paths.libraries.join(maven_path(&library.name)?);
        add_classpath_file(path, &mut classpath, &mut seen)?;
    }
    add_classpath_file(
        state
            .paths
            .versions
            .join(MINECRAFT_VERSION)
            .join(format!("{MINECRAFT_VERSION}.jar")),
        &mut classpath,
        &mut seen,
    )?;
    let classpath = std::env::join_paths(&classpath).map_err(|error| {
        AppError::new(
            AppErrorCode::LaunchFailed,
            "The Minecraft classpath could not be encoded",
        )
        .details(error.to_string())
    })?;
    let temp = state.paths.instance.join("tmp");
    fs::create_dir_all(&temp)
        .map_err(|error| AppError::io("Could not create the instance temp directory", error))?;
    let java_bin = java::windowed_java_executable(Path::new(java_executable));
    let mut command = Command::new(java_bin);
    command
        .arg(format!("-Xms{}M", request.minimum_memory_mb))
        .arg(format!("-Xmx{}M", request.maximum_memory_mb))
        .arg("-XX:+UseG1GC")
        .arg(core_data_dir_argument(&state.paths.root))
        .arg(format!(
            "-Djava.library.path={}",
            state.paths.natives.to_string_lossy()
        ))
        .arg(format!(
            "-Dorg.lwjgl.librarypath={}",
            state.paths.natives.to_string_lossy()
        ))
        .arg(format!("-Djava.io.tmpdir={}", temp.to_string_lossy()))
        .arg("-cp")
        .arg(classpath)
        .arg(&forge.main_class)
        .arg("--username")
        .arg(OFFLINE_USERNAME)
        .arg("--version")
        .arg(FORGE_VERSION_ID)
        .arg("--gameDir")
        .arg(&state.paths.instance)
        .arg("--assetsDir")
        .arg(&state.paths.assets)
        .arg("--assetIndex")
        .arg(vanilla.asset_index.id)
        .arg("--uuid")
        .arg(OFFLINE_UUID)
        .arg("--accessToken")
        .arg("0")
        .arg("--userProperties")
        .arg("{}")
        .arg("--userType")
        .arg("legacy")
        .arg("--width")
        .arg(request.width.to_string())
        .arg("--height")
        .arg(request.height.to_string())
        .arg("--tweakClass")
        .arg("net.minecraftforge.fml.common.launcher.FMLTweaker")
        .current_dir(&state.paths.instance)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false);
    hide_console(&mut command);
    Ok(LaunchSpecification { command })
}

fn add_mojang_library(
    root: &Path,
    library: &MojangLibrary,
    classpath: &mut Vec<PathBuf>,
    seen: &mut BTreeSet<String>,
) -> AppResult<()> {
    if let Some(artifact) = &library.downloads.artifact {
        let relative = artifact.path.as_deref().ok_or_else(|| {
            AppError::new(
                AppErrorCode::InstanceCorrupted,
                "A Minecraft library has no local path",
            )
        })?;
        add_classpath_file(
            root.join(crate::fs_secure::safe_relative_path(relative)?),
            classpath,
            seen,
        )?;
    }
    Ok(())
}

fn add_classpath_file(
    path: PathBuf,
    classpath: &mut Vec<PathBuf>,
    seen: &mut BTreeSet<String>,
) -> AppResult<()> {
    if !path.is_file() {
        return Err(AppError::new(
            AppErrorCode::InstanceCorrupted,
            "A required launch library is missing",
        )
        .details(path.to_string_lossy()));
    }
    let key = path.to_string_lossy().to_ascii_lowercase();
    if seen.insert(key) {
        classpath.push(path);
    }
    Ok(())
}

fn validate_installed_mods(state: &AppState) -> AppResult<()> {
    for installed in crate::mods::list_installed(state)? {
        let path = state.paths.mods.join(&installed.file_name);
        if !path.is_file() {
            return Err(AppError::new(
                AppErrorCode::InstanceCorrupted,
                "An installed mod file is missing",
            )
            .details(installed.file_name));
        }
        let (sha512, _, _) = crate::fs_secure::hash_file(&path)?;
        if !sha512.eq_ignore_ascii_case(&installed.sha512) {
            return Err(AppError::new(
                AppErrorCode::HashMismatch,
                "An installed mod failed local integrity validation",
            )
            .details(installed.name));
        }
    }
    Ok(())
}

async fn drain_output<R>(
    reader: R,
    logger: LocalLogger,
    module: &'static str,
    tail: Arc<Mutex<VecDeque<String>>>,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes).await {
            Ok(0) => break,
            Ok(_) => {
                while matches!(bytes.last(), Some(b'\n' | b'\r')) {
                    bytes.pop();
                }
                // Legacy Minecraft mods sometimes write using the active Windows
                // code page instead of UTF-8. Keep draining the stream and retain a
                // readable, redacted log line instead of losing all subsequent output.
                let line = String::from_utf8_lossy(&bytes);
                let safe = redact(&line);
                logger.info(module, &safe);
                let mut tail = tail.lock();
                if tail.len() >= 200 {
                    tail.pop_front();
                }
                tail.push_back(safe);
            }
            Err(error) => {
                logger.warn(module, format!("Could not read game output: {error}"));
                break;
            }
        }
    }
}

pub fn classify_exit(code: Option<i32>, lines: &[String], stop_requested: bool) -> CrashKind {
    if stop_requested {
        return CrashKind::UserTerminated;
    }
    if code == Some(0) {
        return CrashKind::CleanExit;
    }
    let text = lines.join("\n").to_ascii_lowercase();
    if text.contains("outofmemoryerror") || text.contains("java heap space") {
        CrashKind::OutOfMemory
    } else if text.contains("noclassdeffounderror")
        || text.contains("classnotfoundexception")
        || text.contains("could not find or load main class")
    {
        CrashKind::MissingLibrary
    } else if text.contains("duplicate mod")
        || text.contains("mod sorting data")
        || text.contains("incompatible mod")
    {
        CrashKind::ModConflict
    } else if text.contains("forge mod loader")
        || text.contains("fml has found a non-mod file")
        || text.contains("fml-client-latest")
    {
        CrashKind::ForgeCrash
    } else if text.contains("exception in thread")
        || text.contains("a fatal error has been detected by the java runtime")
    {
        CrashKind::JvmError
    } else {
        CrashKind::Unknown
    }
}

fn set_phase(state: &AppState, phase: LaunchPhase, progress: f32, message: &str) {
    let mut next = state.launch_state();
    next.can_cancel = matches!(
        phase,
        LaunchPhase::Validating
            | LaunchPhase::CheckingRuntime
            | LaunchPhase::PreparingInstance
            | LaunchPhase::VerifyingGameFiles
            | LaunchPhase::InstallingGameFiles
            | LaunchPhase::VerifyingForge
            | LaunchPhase::InstallingForge
            | LaunchPhase::CheckingRequiredMods
            | LaunchPhase::ApplyingPendingChanges
            | LaunchPhase::BuildingLaunchCommand
    );
    next.state = phase;
    next.progress = Some((progress * 100.0).clamp(0.0, 100.0));
    next.message = message.to_owned();
    next.error_id = None;
    next.log_path = None;
    if next.started_at.is_none() {
        next.started_at = Some(Utc::now().to_rfc3339());
    }
    state.set_launch_state(next);
}

fn core_data_dir_argument(root: &Path) -> String {
    format!("-Dprivateclient.dataDir={}", root.to_string_lossy())
}

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.as_std_mut().creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::{
        classify_exit, core_data_dir_argument, stop_decision, taskkill_arguments, StopDecision,
        OFFLINE_USERNAME, OFFLINE_UUID,
    };
    use crate::contracts::CrashKind;

    #[test]
    fn classifies_common_crashes() {
        assert_eq!(
            classify_exit(Some(1), &["java.lang.OutOfMemoryError".to_owned()], false),
            CrashKind::OutOfMemory
        );
        assert_eq!(
            classify_exit(
                Some(1),
                &["java.lang.NoClassDefFoundError: missing".to_owned()],
                false
            ),
            CrashKind::MissingLibrary
        );
        assert_eq!(classify_exit(Some(0), &[], false), CrashKind::CleanExit);
        assert_eq!(classify_exit(Some(1), &[], true), CrashKind::UserTerminated);
    }

    #[test]
    fn offline_bootstrap_uses_non_secret_session_sentinel() {
        // The bundled Core mod is responsible for gating premium/multiplayer
        // features until an in-game session exists. The launcher never owns a
        // Microsoft account token.
        assert_eq!(OFFLINE_USERNAME, "LoginInGame");
        assert_eq!(OFFLINE_UUID, "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn passes_the_launcher_data_root_to_the_core_bridge() {
        assert_eq!(
            core_data_dir_argument(std::path::Path::new(r"C:\Local App Data\Private Client")),
            r"-Dprivateclient.dataDir=C:\Local App Data\Private Client"
        );
    }

    #[test]
    fn stops_the_game_gracefully_before_using_an_exact_forced_tree_fallback() {
        assert_eq!(taskkill_arguments(42, false), vec!["/PID", "42"]);
        assert_eq!(taskkill_arguments(42, true), vec!["/PID", "42", "/T", "/F"]);
    }

    #[test]
    fn stop_policy_forces_any_still_live_verified_process_and_remains_retryable() {
        assert_eq!(stop_decision(false, false), StopDecision::Complete);
        assert_eq!(stop_decision(false, true), StopDecision::Force);
        assert_eq!(stop_decision(true, false), StopDecision::Complete);
        assert_eq!(stop_decision(true, true), StopDecision::Retry);
    }
}
