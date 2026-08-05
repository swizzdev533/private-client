use crate::error::{AppError, AppErrorCode, AppResult};
use crate::logging::redact;
use crate::state::AppState;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub fn open_logs(state: &AppState) -> AppResult<()> {
    let mut command = Command::new("explorer.exe");
    command
        .arg(&state.paths.logs)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .spawn()
        .map_err(|error| AppError::io("Could not open the local logs directory", error))?;
    Ok(())
}

pub fn export_logs(state: &AppState, destination_path: &str) -> AppResult<String> {
    let destination = PathBuf::from(destination_path);
    validate_export_destination(&destination)?;
    let temporary = destination.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let result = (|| -> AppResult<()> {
        let output = File::create(&temporary)
            .map_err(|error| AppError::io("Could not create the log export archive", error))?;
        let mut archive = ZipWriter::new(output);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o600);
        let entries = fs::read_dir(&state.paths.logs)
            .map_err(|error| AppError::io("Could not enumerate local logs", error))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| AppError::io("Could not inspect a local log", error))?;
            let path = entry.path();
            if !path.is_file()
                || path
                    .metadata()
                    .is_ok_and(|metadata| metadata.len() > 20 * 1024 * 1024)
            {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    AppError::new(AppErrorCode::Io, "A local log has an unsupported file name")
                })?;
            archive
                .start_file(name, options)
                .map_err(|error| AppError::io("Could not add a log to the export", error))?;
            let reader = BufReader::new(
                File::open(&path)
                    .map_err(|error| AppError::io("Could not read a local log", error))?,
            );
            for line in reader.lines() {
                let line =
                    line.map_err(|error| AppError::io("Could not read a local log line", error))?;
                writeln!(archive, "{}", redact(&line))
                    .map_err(|error| AppError::io("Could not write a redacted log", error))?;
            }
        }
        archive
            .finish()
            .map_err(|error| AppError::io("Could not finalize the log export", error))?;
        if destination.exists() {
            fs::remove_file(&destination)
                .map_err(|error| AppError::io("Could not replace a previous log export", error))?;
        }
        fs::rename(&temporary, &destination)
            .map_err(|error| AppError::io("Could not commit the log export", error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(destination.to_string_lossy().into_owned())
}

fn validate_export_destination(path: &Path) -> AppResult<()> {
    if !path.is_absolute()
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("zip"))
    {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "The log export destination must be an absolute .zip path",
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            AppErrorCode::InvalidInput,
            "The log export destination has no parent directory",
        )
    })?;
    let metadata = fs::symlink_metadata(parent)
        .map_err(|error| AppError::io("Could not inspect the export directory", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(AppError::new(
            AppErrorCode::SymlinkDetected,
            "The log export directory must be a regular local directory",
        ));
    }
    if path.exists()
        && fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(true)
    {
        return Err(AppError::new(
            AppErrorCode::SymlinkDetected,
            "The log export destination cannot be a symbolic link",
        ));
    }
    Ok(())
}
