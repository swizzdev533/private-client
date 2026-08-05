use crate::error::{AppError, AppResult};
use crate::paths::PathLayout;
use chrono::Utc;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const LOG_LIMIT: u64 = 5 * 1024 * 1024;
const ROTATIONS: usize = 4;

static SECRET_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r"(?i)(authorization|cookie|set-cookie)\s*[:=]\s*[^\r\n]+",
        r"(?i)bearer\s+[A-Za-z0-9._~+/=-]+",
        r"(?i)(authorization|password|access[_ -]?token|refresh[_ -]?token|client[_ -]?secret|cookie|session[_ -]?id)\s*[:=]\s*([^\s,;]+)",
        r"(?i)(accessToken|auth_access_token|auth_session)\s+[^\s]+",
    ]
    .iter()
    .filter_map(|pattern| Regex::new(pattern).ok())
    .collect()
});

#[derive(Clone)]
pub struct LocalLogger {
    path: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl LocalLogger {
    pub fn new(paths: &PathLayout) -> AppResult<Self> {
        fs::create_dir_all(&paths.logs)
            .map_err(|error| AppError::io("Could not initialize local logs", error))?;
        Ok(Self {
            path: paths.logs.join("launcher.log"),
            write_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn info(&self, module: &str, message: impl AsRef<str>) {
        self.write("INFO", module, message.as_ref());
    }

    pub fn warn(&self, module: &str, message: impl AsRef<str>) {
        self.write("WARN", module, message.as_ref());
    }

    pub fn error(&self, module: &str, message: impl AsRef<str>) {
        self.write("ERROR", module, message.as_ref());
    }

    fn write(&self, level: &str, module: &str, message: &str) {
        let _guard = self.write_lock.lock();
        if self
            .path
            .metadata()
            .is_ok_and(|metadata| metadata.len() >= LOG_LIMIT)
        {
            let _ = rotate(&self.path);
        }
        let safe_module = redact(module).replace(['\r', '\n'], " ");
        let safe_message = redact(message).replace('\r', "").replace('\n', "\\n");
        let line = format!(
            "{} [{level}] [{safe_module}] {safe_message}\n",
            Utc::now().to_rfc3339()
        );
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

fn rotate(path: &Path) -> std::io::Result<()> {
    for index in (1..=ROTATIONS).rev() {
        let source = if index == 1 {
            path.to_path_buf()
        } else {
            PathBuf::from(format!("{}.{}", path.display(), index - 1))
        };
        let destination = PathBuf::from(format!("{}.{}", path.display(), index));
        if source.exists() {
            if destination.exists() {
                fs::remove_file(&destination)?;
            }
            fs::rename(source, destination)?;
        }
    }
    Ok(())
}

pub fn redact(value: &str) -> String {
    let mut output = value.to_owned();
    for pattern in SECRET_PATTERNS.iter() {
        output = pattern
            .replace_all(&output, |captures: &regex::Captures<'_>| {
                if captures.len() > 2 {
                    format!(
                        "{}=[REDACTED]",
                        captures.get(1).map_or("secret", |m| m.as_str())
                    )
                } else {
                    "[REDACTED]".to_owned()
                }
            })
            .into_owned();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn redacts_common_secret_shapes() {
        let value = "Authorization: Bearer abc.def password=hunter2 refresh_token=xyz";
        let redacted = redact(value);
        assert!(!redacted.contains("hunter2"));
        assert!(!redacted.contains("abc.def"));
        assert!(!redacted.contains("xyz"));
        assert!(redacted.contains("[REDACTED]"));

        let headers =
            redact("Cookie: session=secret; theme=dark\r\nAuthorization: Basic dXNlcjpwYXNz");
        assert!(!headers.contains("secret"));
        assert!(!headers.contains("dXNlcjpwYXNz"));
    }
}
