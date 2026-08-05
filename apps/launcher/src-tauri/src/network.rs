use crate::error::{AppError, AppErrorCode, AppResult};
use futures_util::StreamExt;
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::redirect::Policy;
use serde::de::DeserializeOwned;
use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::path::Path;
use std::time::Duration;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use url::Url;

const USER_AGENT_VALUE: &str = "PrivateClient/1.0.0 (zero-telemetry launcher)";
const MAX_REDIRECTS: usize = 5;
const JSON_DOWNLOAD_LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DownloadExpectation {
    pub maximum_size: u64,
    pub expected_size: Option<u64>,
    pub sha512: Option<String>,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadReceipt {
    pub size: u64,
    pub sha512: String,
    pub sha1: String,
    pub final_url: String,
}

#[derive(Clone)]
pub struct SecureHttpClient {
    client: reqwest::Client,
}

impl SecureHttpClient {
    pub fn new() -> AppResult<Self> {
        let policy = Policy::custom(|attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                return attempt.error("redirect limit exceeded");
            }
            if validate_url(attempt.url()).is_err() {
                return attempt.error("redirect target is not trusted");
            }
            attempt.follow()
        });
        let client = reqwest::Client::builder()
            .https_only(true)
            .redirect(policy)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .user_agent(USER_AGENT_VALUE)
            .build()
            .map_err(AppError::from)?;
        Ok(Self { client })
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> AppResult<T> {
        let parsed = validate_url_text(url)?;
        let response = self
            .client
            .get(parsed)
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, USER_AGENT_VALUE)
            .send()
            .await
            .map_err(AppError::from)?;
        validate_url(response.url())?;
        if !response.status().is_success() {
            return Err(http_status_error(response.status(), "JSON request"));
        }
        if response
            .content_length()
            .is_some_and(|length| length > JSON_DOWNLOAD_LIMIT)
        {
            return Err(AppError::new(
                AppErrorCode::DownloadTooLarge,
                "The metadata response exceeds the configured size limit",
            ));
        }
        let bytes = response.bytes().await.map_err(AppError::from)?;
        if bytes.len() as u64 > JSON_DOWNLOAD_LIMIT {
            return Err(AppError::new(
                AppErrorCode::DownloadTooLarge,
                "The metadata response exceeds the configured size limit",
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|error| AppError::json("Downloaded metadata is not valid JSON", error))
    }

    pub async fn get_text(&self, url: &str, maximum_size: u64) -> AppResult<String> {
        let parsed = validate_url_text(url)?;
        let response = self
            .client
            .get(parsed)
            .header(ACCEPT, "text/plain, application/json")
            .header(USER_AGENT, USER_AGENT_VALUE)
            .send()
            .await
            .map_err(AppError::from)?;
        validate_url(response.url())?;
        if !response.status().is_success() {
            return Err(http_status_error(response.status(), "text request"));
        }
        if response
            .content_length()
            .is_some_and(|length| length > maximum_size)
        {
            return Err(AppError::new(
                AppErrorCode::DownloadTooLarge,
                "The text response exceeds the configured size limit",
            ));
        }
        let bytes = response.bytes().await.map_err(AppError::from)?;
        if bytes.len() as u64 > maximum_size {
            return Err(AppError::new(
                AppErrorCode::DownloadTooLarge,
                "The text response exceeds the configured size limit",
            ));
        }
        String::from_utf8(bytes.to_vec()).map_err(|error| {
            AppError::new(
                AppErrorCode::ManifestInvalid,
                "The remote text is not valid UTF-8",
            )
            .details(error.to_string())
        })
    }

    pub async fn download(
        &self,
        url: &str,
        destination: &Path,
        expectation: &DownloadExpectation,
    ) -> AppResult<DownloadReceipt> {
        let parsed = validate_url_text(url)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|error| AppError::io("Could not create a download directory", error))?;
        }
        let response = self
            .client
            .get(parsed)
            .header(ACCEPT, "application/octet-stream")
            .header(USER_AGENT, USER_AGENT_VALUE)
            .send()
            .await
            .map_err(AppError::from)?;
        validate_url(response.url())?;
        if !response.status().is_success() {
            return Err(http_status_error(response.status(), "file download"));
        }
        if response
            .content_length()
            .is_some_and(|length| length > expectation.maximum_size)
        {
            return Err(AppError::new(
                AppErrorCode::DownloadTooLarge,
                "The remote file exceeds the configured size limit",
            ));
        }
        let final_url = response.url().to_string();
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(destination)
            .await
            .map_err(|error| AppError::io("Could not create a temporary download", error))?;
        let mut stream = response.bytes_stream();
        let mut sha512 = Sha512::new();
        let mut sha1 = Sha1::new();
        let mut size = 0_u64;
        let result = async {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(AppError::from)?;
                size = size.saturating_add(chunk.len() as u64);
                if size > expectation.maximum_size {
                    return Err(AppError::new(
                        AppErrorCode::DownloadTooLarge,
                        "The downloaded file exceeded its size limit",
                    ));
                }
                file.write_all(&chunk)
                    .await
                    .map_err(|error| AppError::io("Could not stream a downloaded file", error))?;
                sha512.update(&chunk);
                sha1.update(&chunk);
            }
            file.flush()
                .await
                .map_err(|error| AppError::io("Could not flush a downloaded file", error))?;
            file.sync_all()
                .await
                .map_err(|error| AppError::io("Could not synchronize a downloaded file", error))?;
            let actual_sha512 = hex::encode(sha512.finalize());
            let actual_sha1 = hex::encode(sha1.finalize());
            verify_download(size, &actual_sha512, &actual_sha1, expectation)?;
            Ok(DownloadReceipt {
                size,
                sha512: actual_sha512,
                sha1: actual_sha1,
                final_url,
            })
        }
        .await;
        drop(file);
        if result.is_err() {
            let _ = fs::remove_file(destination).await;
        }
        result
    }
}

fn verify_download(
    size: u64,
    sha512: &str,
    sha1: &str,
    expectation: &DownloadExpectation,
) -> AppResult<()> {
    if expectation
        .expected_size
        .is_some_and(|expected| size != expected)
    {
        return Err(AppError::new(
            AppErrorCode::HashMismatch,
            "The downloaded file size does not match provider metadata",
        ));
    }
    if expectation
        .sha512
        .as_deref()
        .is_some_and(|expected| !expected.eq_ignore_ascii_case(sha512))
        || expectation
            .sha1
            .as_deref()
            .is_some_and(|expected| !expected.eq_ignore_ascii_case(sha1))
    {
        return Err(AppError::new(
            AppErrorCode::HashMismatch,
            "The downloaded file hash does not match provider metadata",
        ));
    }
    Ok(())
}

pub fn validate_url_text(value: &str) -> AppResult<Url> {
    let parsed = Url::parse(value)?;
    validate_url(&parsed)?;
    Ok(parsed)
}

pub fn validate_url(url: &Url) -> AppResult<()> {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::new(
            AppErrorCode::UntrustedHost,
            "Only credential-free HTTPS URLs are permitted",
        ));
    }
    if url.port().is_some_and(|port| port != 443) {
        return Err(AppError::new(
            AppErrorCode::UntrustedHost,
            "Non-standard network ports are not permitted",
        ));
    }
    let host = url.host_str().ok_or_else(|| {
        AppError::new(
            AppErrorCode::UntrustedHost,
            "The network destination has no host",
        )
    })?;
    if !is_allowed_host(host) {
        return Err(AppError::new(
            AppErrorCode::UntrustedHost,
            "The network destination is not on the Private Client allowlist",
        )
        .details(host.to_owned()));
    }
    Ok(())
}

pub fn is_allowed_host(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "api.modrinth.com"
            | "cdn.modrinth.com"
            | "piston-meta.mojang.com"
            | "piston-data.mojang.com"
            | "launcher.mojang.com"
            | "launchermeta.mojang.com"
            | "libraries.minecraft.net"
            | "resources.download.minecraft.net"
            | "textures.minecraft.net"
            | "sessionserver.mojang.com"
            | "maven.minecraftforge.net"
            | "files.minecraftforge.net"
            | "optifine.net"
            | "www.optifine.net"
            | "optifined.net"
            | "www.optifined.net"
            | "github.com"
            | "raw.githubusercontent.com"
            | "objects.githubusercontent.com"
            | "release-assets.githubusercontent.com"
            | "codeload.github.com"
    )
}

fn http_status_error(status: reqwest::StatusCode, operation: &str) -> AppError {
    AppError::new(
        AppErrorCode::DownloadFailed,
        format!("The {operation} returned HTTP {}", status.as_u16()),
    )
}

#[cfg(test)]
mod tests {
    use super::{is_allowed_host, validate_url_text, verify_download, DownloadExpectation};

    #[test]
    fn network_allowlist_is_exact() {
        assert!(is_allowed_host("api.modrinth.com"));
        assert!(is_allowed_host("sessionserver.mojang.com"));
        assert!(is_allowed_host("optifine.net"));
        assert!(is_allowed_host("github.com"));
        assert!(is_allowed_host("objects.githubusercontent.com"));
        assert!(is_allowed_host("release-assets.githubusercontent.com"));
        assert!(!is_allowed_host("api.modrinth.com.evil.example"));
        assert!(!is_allowed_host("evil.example"));
        assert!(!is_allowed_host("api.adoptium.org"));
    }

    #[test]
    fn download_expectation_rejects_hash_mismatch() {
        let expectation = DownloadExpectation {
            maximum_size: 1024,
            expected_size: Some(4),
            sha512: Some("aa".repeat(64)),
            sha1: Some("bb".repeat(20)),
        };
        assert!(verify_download(4, &"cc".repeat(64), &"dd".repeat(20), &expectation).is_err());
        assert!(verify_download(4, &"aa".repeat(64), &"bb".repeat(20), &expectation).is_ok());
        assert!(verify_download(5, &"aa".repeat(64), &"bb".repeat(20), &expectation).is_err());
    }

    #[test]
    fn rejects_http_credentials_and_custom_ports() {
        assert!(validate_url_text("http://api.modrinth.com/v2/search").is_err());
        assert!(validate_url_text("https://user@example.com/file").is_err());
        assert!(validate_url_text("https://api.modrinth.com:8443/file").is_err());
        assert!(validate_url_text("https://api.modrinth.com/v2/search").is_ok());
    }
}
