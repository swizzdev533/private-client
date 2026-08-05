use crate::contracts::PathSnapshot;
use crate::error::{AppError, AppErrorCode, AppResult};
use directories::BaseDirs;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PathLayout {
    pub root: PathBuf,
    pub config: PathBuf,
    pub cache: PathBuf,
    pub downloads: PathBuf,
    pub staging: PathBuf,
    pub logs: PathBuf,
    pub profiles: PathBuf,
    pub instance: PathBuf,
    pub versions: PathBuf,
    pub libraries: PathBuf,
    pub assets: PathBuf,
    pub natives: PathBuf,
    pub mods: PathBuf,
    pub runtime: PathBuf,
    pub launcher_settings: PathBuf,
    pub installed_mods: PathBuf,
    pub operation_queue: PathBuf,
    pub instance_state: PathBuf,
    pub profile: PathBuf,
    pub launcher_lock: PathBuf,
    pub game_lock: PathBuf,
    pub game_process: PathBuf,
}

impl PathLayout {
    pub fn discover() -> AppResult<Self> {
        let local = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| BaseDirs::new().map(|dirs| dirs.data_local_dir().to_path_buf()))
            .ok_or_else(|| {
                AppError::new(
                    AppErrorCode::PermissionDenied,
                    "Windows local application data directory is unavailable",
                )
            })?;
        Self::for_root(local.join("Private Client"))
    }

    pub fn for_root(root: PathBuf) -> AppResult<Self> {
        if root.as_os_str().is_empty() || !root.is_absolute() {
            return Err(AppError::new(
                AppErrorCode::InvalidInput,
                "The Private Client data root must be an absolute path",
            ));
        }
        let config = root.join("config");
        let cache = root.join("cache");
        let downloads = cache.join("downloads");
        let staging = root.join("staging");
        let logs = root.join("logs");
        let profiles = root.join("profiles");
        let instance = root.join("instance");
        let versions = instance.join("versions");
        let libraries = instance.join("libraries");
        let assets = instance.join("assets");
        let natives = instance.join("natives").join("1.8.9-forge");
        let mods = instance.join("mods");
        let runtime = root.join("runtime");

        Ok(Self {
            launcher_settings: config.join("launcher-settings.json"),
            installed_mods: config.join("installed-mods.json"),
            operation_queue: config.join("pending-operations.json"),
            instance_state: config.join("instance-state.json"),
            profile: profiles.join("profile.json"),
            launcher_lock: root.join("launcher.lock"),
            game_lock: root.join("game.lock"),
            game_process: config.join("game-process.json"),
            root,
            config,
            cache,
            downloads,
            staging,
            logs,
            profiles,
            instance,
            versions,
            libraries,
            assets,
            natives,
            mods,
            runtime,
        })
    }

    pub fn ensure(&self) -> AppResult<()> {
        let directories = [
            &self.root,
            &self.config,
            &self.cache,
            &self.downloads,
            &self.staging,
            &self.logs,
            &self.profiles,
            &self.instance,
            &self.versions,
            &self.libraries,
            &self.assets,
            &self.natives,
            &self.mods,
            &self.runtime,
        ];
        for directory in directories {
            reject_existing_symlink(directory)?;
            fs::create_dir_all(directory).map_err(|error| {
                AppError::io(
                    format!("Could not create {}", directory.to_string_lossy()),
                    error,
                )
            })?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> PathSnapshot {
        PathSnapshot {
            data_root: display(&self.root),
            instance_root: display(&self.instance),
            logs_directory: display(&self.logs),
            mods_directory: display(&self.mods),
        }
    }
}

fn reject_existing_symlink(path: &Path) -> AppResult<()> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            let metadata = fs::symlink_metadata(candidate).map_err(|error| {
                AppError::io(
                    format!("Could not inspect {}", candidate.to_string_lossy()),
                    error,
                )
            })?;
            if metadata.file_type().is_symlink() {
                return Err(AppError::new(
                    AppErrorCode::SymlinkDetected,
                    "A protected Private Client directory is a symbolic link",
                )
                .details(candidate.to_string_lossy()));
            }
        }
        current = candidate.parent();
    }
    Ok(())
}

pub fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::PathLayout;

    #[test]
    fn creates_isolated_layout_below_root() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("Private Client");
        let paths = PathLayout::for_root(root.clone())?;
        paths.ensure()?;
        assert!(paths.mods.starts_with(&root));
        assert!(paths.mods.is_dir());
        assert!(paths.logs.is_dir());
        Ok(())
    }

    #[test]
    fn rejects_relative_root() {
        let result = PathLayout::for_root("relative".into());
        assert!(result.is_err());
    }
}
