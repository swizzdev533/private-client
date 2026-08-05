use crate::config::ConfigStore;
use crate::contracts::{JavaCandidate, JavaDetection, JavaSource};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::paths::PathLayout;
use crate::runtime_java;
use crate::state::AppState;
use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

const JAVA_PROBE_TIMEOUT: Duration = Duration::from_secs(8);

pub async fn detect(store: &ConfigStore, paths: &PathLayout) -> AppResult<JavaDetection> {
    let configured = store.load_java_config()?.executable;
    let candidates = candidate_paths(&configured, Some(paths));
    let mut inspected = Vec::new();
    for (path, source) in candidates {
        if let Ok(candidate) = inspect_executable(&path, source).await {
            inspected.push(candidate);
        }
    }
    inspected.sort_by_key(|candidate| {
        (
            !candidate.compatible,
            source_rank(&candidate.source),
            candidate.executable.clone(),
        )
    });
    let selected = inspected
        .iter()
        .find(|candidate| candidate.compatible)
        .cloned();
    Ok(JavaDetection {
        selected,
        candidates: inspected,
    })
}

fn source_rank(source: &JavaSource) -> u8 {
    match source {
        JavaSource::Configured => 0,
        JavaSource::JavaHome => 1,
        JavaSource::Path => 2,
        JavaSource::ProgramFiles => 3,
        JavaSource::Managed => 4,
    }
}

pub async fn require_java8(
    store: &ConfigStore,
    paths: &PathLayout,
    override_path: Option<&str>,
) -> AppResult<JavaCandidate> {
    if let Some(value) = override_path.filter(|value| !value.trim().is_empty()) {
        let candidate = inspect_executable(Path::new(value), JavaSource::Configured).await?;
        return validate_compatible_candidate(candidate);
    }
    let detection = detect(store, paths).await?;
    detection.selected.ok_or_else(|| {
        AppError::new(
            AppErrorCode::JavaNotFound,
            "A compatible 64-bit Java 8 runtime was not found",
        )
        .recovery("Install a current 64-bit Java 8 runtime or choose java.exe in settings.")
    })
}

/// Prefer a local compatible Java 8; otherwise download the pinned managed runtime.
pub async fn ensure_java8(state: &AppState) -> AppResult<JavaCandidate> {
    match require_java8(&state.config, &state.paths, None).await {
        Ok(candidate) => Ok(candidate),
        Err(_) => runtime_java::ensure_managed_java8(state).await,
    }
}

pub fn validate_compatible_candidate(candidate: JavaCandidate) -> AppResult<JavaCandidate> {
    if candidate.major != 8 {
        return Err(AppError::new(
            AppErrorCode::JavaIncompatible,
            format!(
                "Minecraft 1.8.9 requires Java 8; found {}",
                candidate.version
            ),
        ));
    }
    if !is_64_bit_architecture(&candidate.architecture) {
        return Err(AppError::new(
            AppErrorCode::JavaArchitectureMismatch,
            "Private Client requires a 64-bit Java 8 runtime",
        ));
    }
    Ok(candidate)
}

fn candidate_paths(configured: &str, paths: Option<&PathLayout>) -> Vec<(PathBuf, JavaSource)> {
    let mut values = Vec::new();
    if !configured.trim().is_empty() {
        values.push((
            windowed_java_executable(Path::new(configured)),
            JavaSource::Configured,
        ));
    }
    if let Some(home) = env::var_os("JAVA_HOME") {
        values.push((java_from_home(&PathBuf::from(home)), JavaSource::JavaHome));
    }
    if let Some(path_value) = env::var_os("PATH") {
        for directory in env::split_paths(&path_value) {
            let candidate = windowed_java_executable(&directory.join("java.exe"));
            if candidate.is_file() {
                values.push((candidate, JavaSource::Path));
            }
        }
    }
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(program_files) = env::var_os(variable) {
            let root = PathBuf::from(program_files);
            for vendor in [
                "Eclipse Adoptium",
                "Java",
                "Microsoft",
                "Amazon Corretto",
                "Zulu",
            ] {
                append_vendor_javas(&root.join(vendor), &mut values);
            }
        }
    }
    if let Some(layout) = paths {
        let managed = runtime_java::managed_javaw_path(layout);
        if managed.is_file() {
            values.push((managed, JavaSource::Managed));
        }
    }
    deduplicate(values)
}

fn append_vendor_javas(root: &Path, output: &mut Vec<(PathBuf, JavaSource)>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten().take(32) {
        let candidate = java_from_home(&entry.path());
        if candidate.is_file() {
            output.push((candidate, JavaSource::ProgramFiles));
        }
    }
}

fn java_from_home(home: &Path) -> PathBuf {
    windowed_java_executable(&home.join("bin").join("java.exe"))
}

pub fn windowed_java_executable(path: &Path) -> PathBuf {
    #[cfg(windows)]
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("java.exe"))
    {
        let javaw = path.with_file_name("javaw.exe");
        if javaw.is_file() {
            return javaw;
        }
    }
    path.to_path_buf()
}

fn probe_java_executable(path: &Path) -> PathBuf {
    #[cfg(windows)]
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("javaw.exe"))
    {
        let java = path.with_file_name("java.exe");
        if java.is_file() {
            return java;
        }
    }
    path.to_path_buf()
}

fn deduplicate(values: Vec<(PathBuf, JavaSource)>) -> Vec<(PathBuf, JavaSource)> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|(path, _)| {
            let key = fs::canonicalize(path)
                .unwrap_or_else(|_| path.to_path_buf())
                .to_string_lossy()
                .to_ascii_lowercase();
            seen.insert(key)
        })
        .collect()
}

pub async fn inspect_executable(path: &Path, source: JavaSource) -> AppResult<JavaCandidate> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| AppError::io("Could not inspect a Java executable", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::new(
            AppErrorCode::JavaNotFound,
            "The Java executable is not a regular local file",
        ));
    }
    let probe_path = probe_java_executable(path);
    let mut command = Command::new(&probe_path);
    command
        .arg("-XshowSettings:properties")
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }
    let output = timeout(JAVA_PROBE_TIMEOUT, command.output())
        .await
        .map_err(|_| {
            AppError::new(
                AppErrorCode::JavaIncompatible,
                "The Java version probe timed out",
            )
        })?
        .map_err(|error| AppError::io("Could not execute the Java version probe", error))?;
    if !output.status.success() {
        return Err(AppError::new(
            AppErrorCode::JavaIncompatible,
            "The Java executable did not complete its version probe",
        ));
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let (version, major) = parse_java_version(&text).ok_or_else(|| {
        AppError::new(
            AppErrorCode::JavaIncompatible,
            "The Java runtime did not report a recognizable version",
        )
    })?;
    let architecture = parse_property(&text, "os.arch").unwrap_or_else(|| "unknown".to_owned());
    let compatible = major == 8 && is_64_bit_architecture(&architecture);
    let target = windowed_java_executable(path);
    let executable = fs::canonicalize(&target).unwrap_or(target);
    Ok(JavaCandidate {
        executable: executable.to_string_lossy().into_owned(),
        version,
        major,
        architecture,
        compatible,
        source,
    })
}

fn parse_java_version(text: &str) -> Option<(String, u16)> {
    let property = parse_property(text, "java.version");
    let quoted = Regex::new(r#"version\s+"([^"]+)""#)
        .ok()
        .and_then(|regex| regex.captures(text))
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_owned()));
    let version = property.or(quoted)?;
    let major_text = if let Some(rest) = version.strip_prefix("1.") {
        rest.split(['.', '_', '-']).next()
    } else {
        version.split(['.', '_', '-']).next()
    }?;
    let major = major_text.parse().ok()?;
    Some((version, major))
}

fn parse_property(text: &str, property: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        if key.trim() == property {
            Some(value.trim().to_owned())
        } else {
            None
        }
    })
}

fn is_64_bit_architecture(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "amd64" | "x86_64" | "aarch64"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        is_64_bit_architecture, parse_java_version, parse_property, probe_java_executable,
        source_rank, windowed_java_executable,
    };
    use crate::contracts::JavaSource;
    use std::path::Path;

    #[test]
    fn parses_legacy_java_8_version() {
        let output = "java.version = 1.8.0_492\nos.arch = amd64\nopenjdk version \"1.8.0_492\"";
        assert_eq!(
            parse_java_version(output),
            Some(("1.8.0_492".to_owned(), 8))
        );
        assert_eq!(parse_property(output, "os.arch").as_deref(), Some("amd64"));
    }

    #[test]
    fn rejects_wrong_architecture() {
        assert!(is_64_bit_architecture("amd64"));
        assert!(!is_64_bit_architecture("x86"));
    }

    #[test]
    fn preserves_non_existent_executable_path() {
        let dummy = Path::new("C:\\nonexistent_dir\\java.exe");
        assert_eq!(windowed_java_executable(dummy), dummy);
        assert_eq!(probe_java_executable(dummy), dummy);
    }

    #[test]
    fn configured_outranks_managed() {
        assert!(source_rank(&JavaSource::Configured) < source_rank(&JavaSource::Managed));
        assert!(source_rank(&JavaSource::ProgramFiles) < source_rank(&JavaSource::Managed));
    }
}
