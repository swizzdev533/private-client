use crate::contracts::{JavaCandidate, JavaSource};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::{ensure_inside, extract_zip_safely, safe_relative_path};
use crate::java;
use crate::network::DownloadExpectation;
use crate::paths::PathLayout;
use crate::state::AppState;
use std::fs;
use std::path::PathBuf;

/// Pinned Eclipse Temurin JRE 8 (Windows x64 Hotspot) for managed installs.
pub const MANAGED_JAVA_VERSION_ID: &str = "8u442-b06";
const MANAGED_JAVA_ARCHIVE_ROOT: &str = "jdk8u442-b06-jre";
const MANAGED_JAVA_URL: &str = "https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u442-b06/OpenJDK8U-jre_x64_windows_hotspot_8u442b06.zip";
const MANAGED_JAVA_FILE_NAME: &str = "OpenJDK8U-jre_x64_windows_hotspot_8u442b06.zip";
const MANAGED_JAVA_SIZE: u64 = 40_653_524;
const MANAGED_JAVA_SHA512: &str = "43727dee79b207eb304fb2d7c6127f5749fadd4cb9e9e50e04fdadb7fac452d656f000b2e0cac5573be595641f306d9738da00d1f49df0a6d156dcfb7cc5c87b";
const MANAGED_JAVA_SHA1: &str = "60403d7651a3545b96a7c914e837e45b6895d131";
const MANAGED_JAVA_LIMIT: u64 = 80 * 1024 * 1024;

pub fn managed_java_home(paths: &PathLayout) -> PathBuf {
    paths
        .runtime
        .join(format!("java8-{MANAGED_JAVA_VERSION_ID}"))
}

pub fn managed_javaw_path(paths: &PathLayout) -> PathBuf {
    managed_java_home(paths).join("bin").join("javaw.exe")
}

/// Returns a probed managed Java 8 if already installed; otherwise downloads,
/// verifies, extracts, and probes the pinned Temurin JRE.
pub async fn ensure_managed_java8(state: &AppState) -> AppResult<JavaCandidate> {
    if let Ok(candidate) = probe_managed(&state.paths).await {
        return Ok(candidate);
    }
    install_managed_java8(state).await?;
    probe_managed(&state.paths).await.map_err(|error| {
        AppError::new(
            AppErrorCode::RuntimeDownloadFailed,
            "The managed Java 8 runtime was installed but failed validation",
        )
        .recovery("Retry PLAY or choose java.exe in settings.")
        .details(error.message)
    })
}

pub async fn probe_managed(paths: &PathLayout) -> AppResult<JavaCandidate> {
    let javaw = managed_javaw_path(paths);
    if !javaw.is_file() {
        return Err(AppError::new(
            AppErrorCode::JavaNotFound,
            "The managed Java 8 runtime is not installed",
        ));
    }
    let candidate = java::inspect_executable(&javaw, JavaSource::Managed).await?;
    java::validate_compatible_candidate(candidate)
}

async fn install_managed_java8(state: &AppState) -> AppResult<()> {
    fs::create_dir_all(&state.paths.runtime)
        .map_err(|error| AppError::io("Could not create the runtime directory", error))?;
    fs::create_dir_all(&state.paths.downloads)
        .map_err(|error| AppError::io("Could not create the downloads directory", error))?;
    fs::create_dir_all(&state.paths.staging)
        .map_err(|error| AppError::io("Could not create the staging directory", error))?;

    let archive = state.paths.downloads.join(MANAGED_JAVA_FILE_NAME);
    if archive.exists() {
        let _ = fs::remove_file(&archive);
    }

    state
        .network
        .download(
            MANAGED_JAVA_URL,
            &archive,
            &DownloadExpectation {
                maximum_size: MANAGED_JAVA_LIMIT,
                expected_size: Some(MANAGED_JAVA_SIZE),
                sha512: Some(MANAGED_JAVA_SHA512.to_owned()),
                sha1: Some(MANAGED_JAVA_SHA1.to_owned()),
            },
        )
        .await
        .map_err(|error| {
            AppError::new(
                AppErrorCode::RuntimeDownloadFailed,
                "Could not download the managed Java 8 runtime",
            )
            .recovery("Check the network connection and retry PLAY.")
            .details(error.to_string())
        })?;

    let extract_root = state
        .paths
        .staging
        .join(format!("java8-{MANAGED_JAVA_VERSION_ID}-extract"));
    if extract_root.exists() {
        fs::remove_dir_all(&extract_root).map_err(|error| {
            AppError::io("Could not reset the Java extraction directory", error)
        })?;
    }
    extract_zip_safely(&archive, &extract_root).map_err(|error| {
        AppError::new(
            AppErrorCode::RuntimeDownloadFailed,
            "Could not extract the managed Java 8 runtime",
        )
        .details(error.to_string())
    })?;

    let archive_home = extract_root.join(MANAGED_JAVA_ARCHIVE_ROOT);
    ensure_inside(&extract_root, &archive_home)?;
    let expected_javaw = archive_home.join("bin").join("javaw.exe");
    if !expected_javaw.is_file() {
        return Err(AppError::new(
            AppErrorCode::RuntimeDownloadFailed,
            "The managed Java archive did not contain javaw.exe",
        )
        .details(expected_javaw.display().to_string()));
    }

    let destination = managed_java_home(&state.paths);
    ensure_inside(&state.paths.runtime, &destination)?;
    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|error| {
            AppError::io("Could not replace the previous managed Java runtime", error)
        })?;
    }
    fs::rename(&archive_home, &destination)
        .map_err(|error| AppError::io("Could not promote the managed Java runtime", error))?;

    let _ = fs::remove_dir_all(&extract_root);
    let _ = fs::remove_file(&archive);
    Ok(())
}

/// Used by unit tests to assert the pinned relative executable path is safe.
pub fn pinned_relative_javaw() -> AppResult<PathBuf> {
    safe_relative_path(&format!("{MANAGED_JAVA_ARCHIVE_ROOT}/bin/javaw.exe"))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{
        pinned_relative_javaw, MANAGED_JAVA_ARCHIVE_ROOT, MANAGED_JAVA_SHA1, MANAGED_JAVA_SHA512,
        MANAGED_JAVA_SIZE, MANAGED_JAVA_URL, MANAGED_JAVA_VERSION_ID,
    };
    use crate::fs_secure::extract_zip_safely;
    use crate::network::is_allowed_host;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use url::Url;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn pin_targets_allowlisted_github_release() {
        let url = Url::parse(MANAGED_JAVA_URL).expect("pin URL");
        assert_eq!(url.scheme(), "https");
        assert!(is_allowed_host(url.host_str().expect("host")));
        assert!(MANAGED_JAVA_URL.contains("jdk8u442-b06"));
        assert_eq!(MANAGED_JAVA_VERSION_ID, "8u442-b06");
        assert_eq!(MANAGED_JAVA_SIZE, 40_653_524);
        assert_eq!(MANAGED_JAVA_SHA512.len(), 128);
        assert_eq!(MANAGED_JAVA_SHA1.len(), 40);
        assert_eq!(MANAGED_JAVA_ARCHIVE_ROOT, "jdk8u442-b06-jre");
    }

    #[test]
    fn pinned_javaw_path_is_relative_and_safe() {
        let path = pinned_relative_javaw().expect("relative");
        assert_eq!(
            path.to_string_lossy().replace('\\', "/"),
            "jdk8u442-b06-jre/bin/javaw.exe"
        );
    }

    #[test]
    fn extracts_expected_javaw_layout_from_fixture_zip() {
        let dir = tempdir().expect("temp");
        let zip_path = dir.path().join("fixture.zip");
        let extract_root = dir.path().join("extract");
        {
            let file = File::create(&zip_path).expect("zip file");
            let mut zip = ZipWriter::new(file);
            let options = SimpleFileOptions::default();
            zip.start_file(
                format!("{MANAGED_JAVA_ARCHIVE_ROOT}/bin/javaw.exe"),
                options,
            )
            .expect("start");
            zip.write_all(b"fake-javaw").expect("write");
            zip.finish().expect("finish");
        }
        extract_zip_safely(&zip_path, &extract_root).expect("extract");
        let javaw: PathBuf = extract_root
            .join(MANAGED_JAVA_ARCHIVE_ROOT)
            .join("bin")
            .join("javaw.exe");
        assert!(javaw.is_file());
        assert_eq!(fs::read(&javaw).expect("read"), b"fake-javaw");
    }
}
