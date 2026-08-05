use crate::error::{AppError, AppErrorCode, AppResult};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;
use zip::ZipArchive;

pub const JSON_LIMIT: u64 = 8 * 1024 * 1024;
pub const JAR_LIMIT: u64 = 512 * 1024 * 1024;

pub fn validate_identifier(value: &str, field: &str) -> AppResult<()> {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if valid && value != "." && value != ".." {
        return Ok(());
    }
    Err(AppError::new(
        AppErrorCode::InvalidInput,
        format!("{field} contains unsupported characters"),
    ))
}

pub fn safe_relative_path(value: &str) -> AppResult<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(path_traversal(value));
    }
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => clean.push(segment),
            _ => return Err(path_traversal(value)),
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(path_traversal(value));
    }
    Ok(clean)
}

fn path_traversal(value: &str) -> AppError {
    AppError::new(
        AppErrorCode::PathTraversalDetected,
        "A path attempted to escape its approved directory",
    )
    .details(value.to_owned())
}

pub fn ensure_inside(root: &Path, candidate: &Path) -> AppResult<()> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| AppError::io("Could not canonicalize the approved directory", error))?;
    let existing = nearest_existing(candidate)?;
    let canonical_existing = fs::canonicalize(&existing)
        .map_err(|error| AppError::io("Could not canonicalize a target path", error))?;
    if !canonical_existing.starts_with(&canonical_root) {
        return Err(path_traversal(&candidate.to_string_lossy()));
    }
    reject_symlink_chain(root, &existing)
}

fn nearest_existing(path: &Path) -> AppResult<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(AppError::new(
                AppErrorCode::PathTraversalDetected,
                "The target path has no existing protected ancestor",
            ));
        }
    }
}

pub fn reject_symlink_chain(root: &Path, path: &Path) -> AppResult<()> {
    let relative = path.strip_prefix(root).map_err(|_| {
        AppError::new(
            AppErrorCode::PathTraversalDetected,
            "The target is outside the approved directory",
        )
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if current.exists() {
            let metadata = fs::symlink_metadata(&current)
                .map_err(|error| AppError::io("Could not inspect a protected path", error))?;
            if metadata.file_type().is_symlink() {
                return Err(AppError::new(
                    AppErrorCode::SymlinkDetected,
                    "Symbolic links are not allowed in protected paths",
                )
                .details(current.to_string_lossy()));
            }
        }
    }
    Ok(())
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> AppResult<T> {
    let file = File::open(path)
        .map_err(|error| AppError::io(format!("Could not open {}", path.display()), error))?;
    let metadata = file
        .metadata()
        .map_err(|error| AppError::io("Could not inspect a JSON document", error))?;
    if metadata.len() > JSON_LIMIT {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            "A local JSON document exceeds the size limit",
        ));
    }
    serde_json::from_reader(BufReader::new(file))
        .map_err(|error| AppError::json(format!("Could not parse {}", path.display()), error))
}

pub fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| AppError::json("Could not serialize local data", error))?;
    atomic_write(path, &bytes)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            AppErrorCode::InvalidInput,
            "The destination has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| AppError::io("Could not create the destination directory", error))?;
    let temporary = parent.join(format!(".{}.tmp", Uuid::new_v4()));
    let result = (|| -> AppResult<()> {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| AppError::io("Could not create a temporary file", error))?;
        let mut writer = BufWriter::new(file);
        writer
            .write_all(bytes)
            .map_err(|error| AppError::io("Could not write a temporary file", error))?;
        writer
            .flush()
            .map_err(|error| AppError::io("Could not flush a temporary file", error))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|error| AppError::io("Could not synchronize a temporary file", error))?;
        drop(writer);
        replace_file(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn atomic_copy(source: &Path, destination: &Path) -> AppResult<()> {
    let parent = destination.parent().ok_or_else(|| {
        AppError::new(
            AppErrorCode::InvalidInput,
            "The copy destination has no parent",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| AppError::io("Could not create the copy destination", error))?;
    let temporary = parent.join(format!(".{}.copy", Uuid::new_v4()));
    let result = (|| -> AppResult<()> {
        fs::copy(source, &temporary)
            .map_err(|error| AppError::io("Could not copy a local file", error))?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| AppError::io("Could not reopen the copied file", error))?;
        file.sync_all()
            .map_err(|error| AppError::io("Could not synchronize the copied file", error))?;
        replace_file(&temporary, destination)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn replace_file(source: &Path, destination: &Path) -> AppResult<()> {
    if destination.exists() {
        fs::remove_file(destination)
            .map_err(|error| AppError::io("Could not replace an existing file", error))?;
    }
    fs::rename(source, destination)
        .map_err(|error| AppError::io("Could not commit an atomic file operation", error))
}

pub fn hash_file(path: &Path) -> AppResult<(String, String, u64)> {
    let file = File::open(path)
        .map_err(|error| AppError::io(format!("Could not hash {}", path.display()), error))?;
    let mut reader = BufReader::new(file);
    let mut sha512 = Sha512::new();
    let mut sha1 = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut size = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| AppError::io("Could not read a file for verification", error))?;
        if read == 0 {
            break;
        }
        size = size.saturating_add(read as u64);
        sha512.update(&buffer[..read]);
        sha1.update(&buffer[..read]);
    }
    Ok((
        hex::encode(sha512.finalize()),
        hex::encode(sha1.finalize()),
        size,
    ))
}

pub fn hash_bytes(bytes: &[u8]) -> (String, String) {
    let mut sha512 = Sha512::new();
    let mut sha1 = Sha1::new();
    sha512.update(bytes);
    sha1.update(bytes);
    (hex::encode(sha512.finalize()), hex::encode(sha1.finalize()))
}

pub fn validate_jar(path: &Path, maximum_size: u64) -> AppResult<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| AppError::io("Could not inspect the JAR", error))?;
    if metadata.file_type().is_symlink() {
        return Err(AppError::new(
            AppErrorCode::SymlinkDetected,
            "A symbolic link cannot be imported as a mod",
        ));
    }
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum_size {
        return Err(AppError::new(
            AppErrorCode::JarValidationFailed,
            "The JAR size is invalid",
        ));
    }
    if path
        .extension()
        .and_then(OsStr::to_str)
        .map(|extension| !extension.eq_ignore_ascii_case("jar"))
        .unwrap_or(true)
    {
        return Err(AppError::new(
            AppErrorCode::JarValidationFailed,
            "The selected file must use the .jar extension",
        ));
    }
    let file = File::open(path).map_err(|error| AppError::io("Could not open the JAR", error))?;
    let mut archive = ZipArchive::new(file)?;
    if archive.is_empty() || archive.len() > 65_536 {
        return Err(AppError::new(
            AppErrorCode::JarValidationFailed,
            "The JAR has an invalid entry count",
        ));
    }
    let mut recognizable = false;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name();
        safe_relative_path(name)?;
        if matches!(
            name.to_ascii_lowercase().as_str(),
            "mcmod.info" | "meta-inf/mods.toml" | "fabric.mod.json" | "meta-inf/manifest.mf"
        ) {
            recognizable = true;
        }
    }
    if !recognizable {
        return Err(AppError::new(
            AppErrorCode::JarValidationFailed,
            "The archive does not contain recognizable mod metadata",
        ));
    }
    Ok(())
}

pub fn extract_zip_safely(source: &Path, destination: &Path) -> AppResult<()> {
    fs::create_dir_all(destination)
        .map_err(|error| AppError::io("Could not create the extraction directory", error))?;
    let file =
        File::open(source).map_err(|error| AppError::io("Could not open the archive", error))?;
    let mut archive = ZipArchive::new(file)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = safe_relative_path(entry.name())?;
        if relative
            .components()
            .next()
            .and_then(|component| component.as_os_str().to_str())
            .is_some_and(|first| first.eq_ignore_ascii_case("META-INF"))
        {
            continue;
        }
        let output = destination.join(relative);
        ensure_inside(destination, &output)?;
        if entry.is_dir() {
            fs::create_dir_all(&output)
                .map_err(|error| AppError::io("Could not create an archive directory", error))?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    AppError::io("Could not create an archive parent directory", error)
                })?;
            }
            let mut writer = BufWriter::new(
                File::create(&output)
                    .map_err(|error| AppError::io("Could not extract an archive entry", error))?,
            );
            std::io::copy(&mut entry, &mut writer)
                .map_err(|error| AppError::io("Could not extract an archive entry", error))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        atomic_copy, atomic_write, ensure_inside, safe_relative_path, validate_identifier,
    };

    #[test]
    fn blocks_traversal_and_absolute_paths() {
        assert!(safe_relative_path("../escape.jar").is_err());
        assert!(safe_relative_path("mods/../../escape.jar").is_err());
        assert!(safe_relative_path("C:\\Windows\\file").is_err());
        assert!(safe_relative_path("mods/safe.jar").is_ok());
    }

    #[test]
    fn validates_external_identifiers() {
        assert!(validate_identifier("A1-b_C.9", "id").is_ok());
        assert!(validate_identifier("../oops", "id").is_err());
        assert!(validate_identifier("", "id").is_err());
    }

    #[test]
    fn ensures_target_remains_inside_root() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("root");
        std::fs::create_dir_all(&root)?;
        assert!(ensure_inside(&root, &root.join("a").join("file")).is_ok());
        assert!(ensure_inside(&root, &directory.path().join("outside")).is_err());
        Ok(())
    }

    #[test]
    fn atomically_copies_and_synchronizes_a_local_file() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("source.jar");
        let destination = directory.path().join("destination.jar");
        atomic_write(&source, b"verified payload")?;
        atomic_copy(&source, &destination)?;
        assert_eq!(std::fs::read(destination)?, b"verified payload");
        Ok(())
    }
}
