use crate::contracts::{JavaConfig, LauncherSettings};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::{atomic_write_json, read_json};
use crate::paths::PathLayout;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::System;

#[derive(Clone)]
pub struct ConfigStore {
    paths: PathLayout,
}

impl ConfigStore {
    pub fn new(paths: PathLayout) -> Self {
        Self { paths }
    }

    pub fn load_settings(&self) -> AppResult<LauncherSettings> {
        if !self.paths.launcher_settings.exists() {
            return Ok(LauncherSettings::default());
        }
        let mut settings: LauncherSettings = read_json(&self.paths.launcher_settings)?;
        if settings.schema_version == 0 {
            settings.schema_version = 1;
        }
        validate_settings(&settings, Some(&self.paths.runtime))?;
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &LauncherSettings) -> AppResult<()> {
        validate_settings(settings, Some(&self.paths.runtime))?;
        validate_memory_for_system(settings.memory_max_mb)?;
        atomic_write_json(&self.paths.launcher_settings, settings)
    }

    pub fn load_java_config(&self) -> AppResult<JavaConfig> {
        let settings = self.load_settings()?;
        Ok(JavaConfig {
            executable: settings.java_path.unwrap_or_default(),
            minimum_memory_mb: settings.memory_min_mb,
            maximum_memory_mb: settings.memory_max_mb,
        })
    }

    pub fn save_java_config(&self, config: &JavaConfig) -> AppResult<()> {
        validate_memory(config.minimum_memory_mb, config.maximum_memory_mb)?;
        validate_memory_for_system(config.maximum_memory_mb)?;
        validate_java_path_if_present(&config.executable, Some(&self.paths.runtime))?;
        let mut settings = self.load_settings()?;
        settings.java_path = if config.executable.trim().is_empty() {
            None
        } else {
            Some(config.executable.clone())
        };
        settings.memory_min_mb = config.minimum_memory_mb;
        settings.memory_max_mb = config.maximum_memory_mb;
        self.save_settings(&settings)
    }
}

pub fn validate_settings(settings: &LauncherSettings, runtime: Option<&Path>) -> AppResult<()> {
    if settings.schema_version != 1 {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "The launcher settings schema is unsupported",
        ));
    }
    validate_memory(settings.memory_min_mb, settings.memory_max_mb)?;
    validate_java_path_if_present(settings.java_path.as_deref().unwrap_or_default(), runtime)?;
    if !(1..=8).contains(&settings.download_concurrency) {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "Download concurrency must be between one and eight",
        ));
    }
    Ok(())
}

pub fn validate_memory(minimum: u32, maximum: u32) -> AppResult<()> {
    if !(512..=32_768).contains(&minimum)
        || !(1024..=32_768).contains(&maximum)
        || minimum > maximum
    {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "The memory allocation must be between 512 MiB and 32 GiB",
        )
        .recovery("Choose a maximum value greater than or equal to the minimum value."));
    }
    Ok(())
}

pub fn validate_memory_for_system(maximum: u32) -> AppResult<()> {
    let (total_mb, safe_maximum) = system_memory_limits();
    validate_memory_against_total(maximum, total_mb).map_err(|error| {
        error.recovery(format!(
            "Choose at most {safe_maximum} MiB on this computer ({total_mb} MiB installed)."
        ))
    })
}

pub fn system_memory_limits() -> (u32, u32) {
    let mut system = System::new();
    system.refresh_memory();
    let total_mb = u32::try_from(system.total_memory() / (1024 * 1024)).unwrap_or(u32::MAX);
    let safe_maximum = memory_ceiling(total_mb);
    (total_mb, safe_maximum)
}

fn validate_memory_against_total(maximum: u32, total_mb: u32) -> AppResult<()> {
    let safe_maximum = memory_ceiling(total_mb);
    if maximum > safe_maximum {
        return Err(AppError::new(
            AppErrorCode::InsufficientMemory,
            "The selected game memory would leave too little RAM for Windows",
        ));
    }
    Ok(())
}

fn memory_ceiling(total_mb: u32) -> u32 {
    let half_of_system_memory = total_mb / 2;
    let after_reserve = total_mb.saturating_sub(2048);
    half_of_system_memory.min(after_reserve)
}

/// Roots a frontend-supplied Java executable is allowed to live under.
///
/// A configured `java_path` is executed as a program (`java::inspect_executable`
/// probes it, and `JavaSource::Configured` outranks every autodetected
/// candidate, so it is also the binary the game is launched with). The webview
/// is untrusted, so the path is confined to the managed runtime directory and
/// the system install locations rather than being accepted anywhere on disk.
fn approved_java_roots(runtime: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(runtime) = runtime {
        roots.push(runtime.to_path_buf());
    }
    for variable in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Some(value) = std::env::var_os(variable) {
            roots.push(PathBuf::from(value));
        }
    }
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        roots.push(PathBuf::from(home));
    }
    roots
        .into_iter()
        .filter_map(|root| fs::canonicalize(root).ok())
        .collect()
}

fn validate_java_path_if_present(value: &str, runtime: Option<&Path>) -> AppResult<()> {
    if value.trim().is_empty() {
        return Ok(());
    }
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(AppError::new(
            AppErrorCode::JavaNotFound,
            "The configured Java executable must be an absolute path",
        ));
    }
    // UNC and device paths can redirect to a remote or arbitrary namespace that
    // canonicalization will happily accept, so reject them outright.
    if value.starts_with("\\\\") || value.starts_with("//") {
        return Err(AppError::new(
            AppErrorCode::JavaNotFound,
            "The configured Java executable must be on a local drive",
        ));
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        AppError::io("The configured Java executable cannot be inspected", error)
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::new(
            AppErrorCode::JavaNotFound,
            "The configured Java executable is not a regular file",
        ));
    }
    let accepted = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case("java.exe")
                || name.eq_ignore_ascii_case("javaw.exe")
                || name.eq_ignore_ascii_case("java")
        });
    if !accepted {
        return Err(AppError::new(
            AppErrorCode::JavaNotFound,
            "The configured path does not point to java.exe or javaw.exe",
        ));
    }
    // Canonicalize after the symlink check so every intermediate component is
    // resolved before the path is compared against the allowlist.
    let resolved = fs::canonicalize(path).map_err(|error| {
        AppError::io("The configured Java executable cannot be resolved", error)
    })?;
    let roots = approved_java_roots(runtime);
    if !roots.iter().any(|root| resolved.starts_with(root)) {
        return Err(AppError::new(
            AppErrorCode::JavaNotFound,
            "The configured Java executable is outside the approved installation directories",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{
        validate_java_path_if_present, validate_memory, validate_memory_against_total,
        validate_settings,
    };
    use crate::contracts::LauncherSettings;
    use std::fs;

    #[test]
    fn rejects_invalid_memory_ranges() {
        assert!(validate_memory(2048, 1024).is_err());
        assert!(validate_memory(128, 2048).is_err());
        assert!(validate_memory(512, 2048).is_ok());
    }

    #[test]
    fn default_settings_are_valid() {
        assert!(validate_settings(&LauncherSettings::default(), None).is_ok());
    }

    #[test]
    fn keeps_a_system_memory_reserve() {
        assert!(validate_memory_against_total(6144, 8192).is_err());
        assert!(validate_memory_against_total(4096, 8192).is_ok());
    }

    #[test]
    fn rejects_a_java_executable_outside_the_approved_roots() {
        let scratch = std::env::temp_dir().join("private-client-java-path-test");
        fs::create_dir_all(&scratch).expect("scratch directory");
        let planted = scratch.join("java.exe");
        fs::write(&planted, b"not really java").expect("planted executable");

        let value = planted.to_string_lossy().into_owned();
        // Correct file name, real regular file, absolute path - and still refused
        // because it does not resolve under the managed runtime or an install root.
        assert!(validate_java_path_if_present(&value, None).is_err());

        let settings = LauncherSettings {
            java_path: Some(value.clone()),
            ..LauncherSettings::default()
        };
        assert!(validate_settings(&settings, None).is_err());

        // The same binary is accepted once it sits inside the managed runtime root.
        assert!(validate_java_path_if_present(&value, Some(&scratch)).is_ok());

        fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn rejects_relative_and_unc_java_paths() {
        assert!(validate_java_path_if_present("java.exe", None).is_err());
        assert!(validate_java_path_if_present(r"..\java.exe", None).is_err());
        assert!(validate_java_path_if_present(r"\\attacker\share\java.exe", None).is_err());
    }

    #[test]
    fn an_absent_java_path_stays_optional() {
        assert!(validate_java_path_if_present("", None).is_ok());
        assert!(validate_java_path_if_present("   ", None).is_ok());
    }
}
