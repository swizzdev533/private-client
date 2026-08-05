use crate::contracts::{LocalProfile, SkinModel, EVENT_PROFILE_UPDATED};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::{
    atomic_copy, atomic_write_json, ensure_inside, read_json, reject_symlink_chain,
    safe_relative_path,
};
use crate::logging::LocalLogger;
use crate::network::{DownloadExpectation, SecureHttpClient};
use crate::paths::PathLayout;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use regex::Regex;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

const SKIN_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct SessionProfile {
    #[serde(default)]
    properties: Vec<SessionProperty>,
}

#[derive(Debug, Deserialize)]
struct SessionProperty {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct TexturePayload {
    textures: TextureMap,
}

#[derive(Debug, Deserialize)]
struct TextureMap {
    #[serde(rename = "SKIN")]
    skin: Option<SkinTexture>,
}

#[derive(Debug, Deserialize)]
struct SkinTexture {
    url: String,
    metadata: Option<SkinMetadata>,
}

#[derive(Debug, Deserialize)]
struct SkinMetadata {
    model: Option<String>,
}

pub fn read(paths: &PathLayout) -> AppResult<Option<LocalProfile>> {
    read_stored(paths)?
        .map(|profile| public_profile(paths, profile))
        .transpose()
}

fn read_stored(paths: &PathLayout) -> AppResult<Option<LocalProfile>> {
    if !paths.profile.exists() {
        return Ok(None);
    }
    let profile: LocalProfile = read_json(&paths.profile).map_err(|error| {
        AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The local profile cache is invalid",
        )
        .details(error.to_string())
    })?;
    validate(&profile)?;
    Ok(Some(profile))
}

fn public_profile(paths: &PathLayout, mut profile: LocalProfile) -> AppResult<LocalProfile> {
    if let Some(relative) = profile
        .skin_path
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        let absolute = paths.root.join(safe_relative_path(relative)?);
        ensure_inside(&paths.root, &absolute)?;
        profile.skin_path = Some(absolute.to_string_lossy().into_owned());
    } else {
        profile.skin_path = None;
    }
    Ok(profile)
}

pub fn validate(profile: &LocalProfile) -> AppResult<()> {
    if profile.schema_version != 1 {
        return Err(AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The local profile schema is unsupported",
        ));
    }
    let username_pattern = Regex::new(r"^[A-Za-z0-9_]{1,16}$").map_err(|error| {
        AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "Could not initialize profile validation",
        )
        .details(error.to_string())
    })?;
    if !username_pattern.is_match(&profile.username) {
        return Err(AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The cached Minecraft username is invalid",
        ));
    }
    uuid::Uuid::parse_str(&profile.uuid).map_err(|error| {
        AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The cached Minecraft UUID is invalid",
        )
        .details(error.to_string())
    })?;
    if let Some(skin_path) = profile
        .skin_path
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        let expected = format!("cache/profiles/{}/skin.png", profile.uuid);
        if skin_path.replace('\\', "/") != expected {
            return Err(AppError::new(
                AppErrorCode::ProfileCacheInvalid,
                "The cached skin path is outside the profile cache",
            ));
        }
    }
    Ok(())
}

pub fn start_watcher(
    app: AppHandle,
    paths: PathLayout,
    logger: LocalLogger,
    network: SecureHttpClient,
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
) -> bool {
    if running.swap(true, Ordering::SeqCst) {
        return false;
    }
    stop.store(false, Ordering::SeqCst);
    let skin_refresh_running = Arc::new(AtomicBool::new(false));
    tauri::async_runtime::spawn(async move {
        let mut fingerprint = file_fingerprint(&paths);
        handle_profile_change(&app, &paths, &logger, &network, &skin_refresh_running);
        while !stop.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(750)).await;
            let next = file_fingerprint(&paths);
            if next != fingerprint {
                fingerprint = next;
                handle_profile_change(&app, &paths, &logger, &network, &skin_refresh_running);
            }
        }
        running.store(false, Ordering::SeqCst);
    });
    true
}

fn handle_profile_change(
    app: &AppHandle,
    paths: &PathLayout,
    logger: &LocalLogger,
    network: &SecureHttpClient,
    refresh_running: &Arc<AtomicBool>,
) {
    match read(paths) {
        Ok(Some(profile)) => {
            let _ = app.emit(EVENT_PROFILE_UPDATED, &profile);
            if needs_skin_refresh(paths, &profile) && !refresh_running.swap(true, Ordering::SeqCst)
            {
                let app = app.clone();
                let paths = paths.clone();
                let logger = logger.clone();
                let network = network.clone();
                let refresh_running = Arc::clone(refresh_running);
                tauri::async_runtime::spawn(async move {
                    match refresh_skin(&paths, &network).await {
                        Ok(Some(profile)) => {
                            let _ = app.emit(EVENT_PROFILE_UPDATED, profile);
                        }
                        Ok(None) => {}
                        Err(error) => logger.warn("profile.skin", error.to_string()),
                    }
                    refresh_running.store(false, Ordering::SeqCst);
                });
            }
        }
        Ok(None) => {
            let _ = app.emit(EVENT_PROFILE_UPDATED, Option::<LocalProfile>::None);
        }
        Err(error) => logger.warn("profile", error.to_string()),
    }
}

fn needs_skin_refresh(paths: &PathLayout, profile: &LocalProfile) -> bool {
    let Some(path) = profile
        .skin_path
        .as_deref()
        .filter(|value| !value.is_empty())
    else {
        return true;
    };
    validate_png(&paths.root.join(path)).is_err()
}

async fn refresh_skin(
    paths: &PathLayout,
    network: &SecureHttpClient,
) -> AppResult<Option<LocalProfile>> {
    let Some(original) = read_stored(paths)? else {
        return Ok(None);
    };
    let uuid = Uuid::parse_str(&original.uuid).map_err(|error| {
        AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The profile UUID is invalid",
        )
        .details(error.to_string())
    })?;
    let compact_uuid = uuid.simple().to_string();
    let session: SessionProfile = network
        .get_json(&format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{compact_uuid}?unsigned=false"
        ))
        .await?;
    let property = session
        .properties
        .iter()
        .find(|property| property.name == "textures")
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::ProfileCacheInvalid,
                "The official profile has no textures property",
            )
        })?;
    if property.value.len() > 128 * 1024 {
        return Err(AppError::new(
            AppErrorCode::DownloadTooLarge,
            "The official texture payload is too large",
        ));
    }
    let decoded = STANDARD.decode(&property.value).map_err(|error| {
        AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The official texture payload is not valid Base64",
        )
        .details(error.to_string())
    })?;
    let payload: TexturePayload = serde_json::from_slice(&decoded)
        .map_err(|error| AppError::json("The official texture payload is invalid", error))?;
    let skin = payload.textures.skin.ok_or_else(|| {
        AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The official profile has no skin texture",
        )
    })?;
    let skin_url = validate_skin_url(&skin.url)?;
    let relative = format!("cache/profiles/{}/skin.png", original.uuid);
    let destination = paths.root.join(safe_relative_path(&relative)?);
    ensure_inside(&paths.root, &destination)?;
    reject_symlink_chain(&paths.root, &destination)?;
    let staging = paths.staging.join(format!("skin-{}.png", Uuid::new_v4()));
    ensure_inside(&paths.staging, &staging)?;
    let result = async {
        network
            .download(
                skin_url.as_str(),
                &staging,
                &DownloadExpectation {
                    maximum_size: SKIN_LIMIT,
                    expected_size: None,
                    sha512: None,
                    sha1: None,
                },
            )
            .await?;
        validate_png(&staging)?;
        atomic_copy(&staging, &destination)
    }
    .await;
    let _ = fs::remove_file(&staging);
    result?;

    let Some(mut current) = read_stored(paths)? else {
        return Ok(None);
    };
    if current.uuid != original.uuid {
        return Ok(None);
    }
    current.skin_path = Some(relative);
    current.skin_model = if skin
        .metadata
        .and_then(|metadata| metadata.model)
        .is_some_and(|model| model.eq_ignore_ascii_case("slim"))
    {
        SkinModel::Slim
    } else {
        SkinModel::Classic
    };
    current.updated_at = Utc::now().to_rfc3339();
    atomic_write_json(&paths.profile, &current)?;
    Ok(Some(public_profile(paths, current)?))
}

fn validate_skin_url(value: &str) -> AppResult<url::Url> {
    let mut url = url::Url::parse(value)?;
    let path_ok = url.path_segments().is_some_and(|mut segments| {
        segments.next() == Some("texture")
            && segments.next().is_some_and(|hash| {
                !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            && segments.next().is_none()
    });
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.host_str() != Some("textures.minecraft.net")
        || !path_ok
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(AppError::new(
            AppErrorCode::UntrustedHost,
            "The official skin response points outside the Minecraft texture service",
        ));
    }
    url.set_scheme("https").map_err(|_| {
        AppError::new(
            AppErrorCode::UntrustedHost,
            "The official skin URL could not be upgraded to HTTPS",
        )
    })?;
    crate::network::validate_url(&url)?;
    Ok(url)
}

fn validate_png(path: &std::path::Path) -> AppResult<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| AppError::io("Could not inspect the cached skin", error))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() < 24
        || metadata.len() > SKIN_LIMIT
    {
        return Err(AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The cached skin is not a safe PNG file",
        ));
    }
    let mut header = [0_u8; 24];
    File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|error| AppError::io("Could not read the cached skin header", error))?;
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    if &header[..8] != b"\x89PNG\r\n\x1a\n"
        || &header[12..16] != b"IHDR"
        || width != 64
        || !matches!(height, 32 | 64)
    {
        return Err(AppError::new(
            AppErrorCode::ProfileCacheInvalid,
            "The cached skin has an invalid PNG header",
        ));
    }
    Ok(())
}

fn file_fingerprint(paths: &PathLayout) -> Option<(u64, SystemTime)> {
    fs::metadata(&paths.profile)
        .ok()
        .and_then(|metadata| Some((metadata.len(), metadata.modified().ok()?)))
}

#[cfg(test)]
mod tests {
    use super::{validate, validate_skin_url, SessionProfile, TexturePayload};
    use crate::contracts::{LocalProfile, SkinModel};
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    #[test]
    fn validates_profile_without_tokens() {
        let profile = LocalProfile {
            schema_version: 1,
            username: "Player_Name".to_owned(),
            uuid: "8667ba71-b85a-4004-af54-457a9734eed7".to_owned(),
            skin_path: None,
            skin_model: SkinModel::Classic,
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        assert!(validate(&profile).is_ok());
    }

    #[test]
    fn rejects_invalid_username() {
        let profile = LocalProfile {
            schema_version: 1,
            username: "../bad".to_owned(),
            uuid: "8667ba71-b85a-4004-af54-457a9734eed7".to_owned(),
            skin_path: None,
            skin_model: SkinModel::Classic,
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        assert!(validate(&profile).is_err());
    }

    #[test]
    fn deleted_profile_event_serializes_as_null() -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_value(Option::<LocalProfile>::None)?;
        assert!(payload.is_null());
        Ok(())
    }

    #[test]
    fn parses_official_session_and_texture_payloads_without_tokens(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let texture_json = r#"{"textures":{"SKIN":{"url":"https://textures.minecraft.net/texture/abc","metadata":{"model":"slim"}}}}"#;
        let session_json = format!(
            r#"{{"properties":[{{"name":"textures","value":"{}"}}]}}"#,
            STANDARD.encode(texture_json)
        );
        let session: SessionProfile = serde_json::from_str(&session_json)?;
        let decoded = STANDARD.decode(&session.properties[0].value)?;
        let payload: TexturePayload = serde_json::from_slice(&decoded)?;
        let skin = payload.textures.skin.ok_or("missing skin")?;
        assert!(validate_skin_url(&skin.url).is_ok());
        assert_eq!(
            validate_skin_url("http://textures.minecraft.net/texture/abcdef")?.scheme(),
            "https"
        );
        assert_eq!(
            skin.metadata.and_then(|metadata| metadata.model).as_deref(),
            Some("slim")
        );
        assert!(validate_skin_url("https://textures.minecraft.net.evil.test/texture/a").is_err());
        Ok(())
    }
}
