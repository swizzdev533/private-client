use crate::contracts::{
    ImportOptifineRequest, InstalledMod, ModCompatibility, ModEnvironment, ModOperationResult,
    ModSource, ModTrust, ReleaseType,
};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::fs_secure::{atomic_copy, hash_file, validate_jar};
use crate::state::AppState;
use chrono::Utc;
use std::fs::{self, File};
use std::path::Path;
use uuid::Uuid;
use zip::ZipArchive;

const OPTIFINE_LIMIT: u64 = 64 * 1024 * 1024;
const OPTIFINE_FILE_NAME: &str = "OptiFine_1.8.9_HD_U_M5.jar";
const OPTIFINE_SIZE: u64 = 2_585_014;
const OPTIFINE_SHA1: &str = "d362d58a28f5373b141b9e426e8e160638bfafcd";
const OPTIFINE_SHA512: &str = "459dd48fd88cbf3e91e7b02790e272279465978a5bde8e85409afa28cd1b0c9022195588afcc2f63acdec5688ac3e93adf7e9870da28548c3b1c292a598426de";

pub async fn download_and_import(state: &AppState) -> AppResult<ModOperationResult> {
    let _guard = state.operation_lock.lock().await;
    // Checked under the lock so a launch that starts while we wait cannot be
    // raced - see the note in `import`.
    if state.is_game_running() {
        return Err(AppError::new(
            AppErrorCode::OperationBlockedWhileRunning,
            "Private Pack cannot be installed while Minecraft is running",
        ));
    }
    let staging_path = state.paths.staging.join(OPTIFINE_FILE_NAME);
    if staging_path.exists() {
        tokio::fs::remove_file(&staging_path)
            .await
            .map_err(|error| AppError::io("Could not reset OptiFine staging", error))?;
    }
    let landing = state
        .network
        .get_text(
            "https://optifine.net/adloadx?f=OptiFine_1.8.9_HD_U_M5.jar",
            2 * 1024 * 1024,
        )
        .await?;
    let token_pattern = regex::Regex::new(
        r#"downloadx\?f=OptiFine_1\.8\.9_HD_U_M5\.jar(?:&amp;|&)x=([a-fA-F0-9]{32})"#,
    )
    .map_err(|error| {
        AppError::new(
            AppErrorCode::ManifestInvalid,
            "Invalid OptiFine download parser",
        )
        .details(error.to_string())
    })?;
    let token = token_pattern
        .captures(&landing)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str())
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::DownloadFailed,
                "Official OptiFine page did not expose the expected 1.8.9 download",
            )
        })?;
    let download_url = format!("https://optifine.net/downloadx?f={OPTIFINE_FILE_NAME}&x={token}");
    state
        .network
        .download(
            &download_url,
            &staging_path,
            &crate::network::DownloadExpectation {
                maximum_size: OPTIFINE_LIMIT,
                expected_size: Some(OPTIFINE_SIZE),
                sha512: Some(OPTIFINE_SHA512.to_owned()),
                sha1: Some(OPTIFINE_SHA1.to_owned()),
            },
        )
        .await?;
    let record = import_now(state, &staging_path)?;
    install_external_components(state).await?;
    let _ = tokio::fs::remove_file(&staging_path).await;
    Ok(ModOperationResult {
        operation_id: Uuid::new_v4().to_string(),
        queued: false,
        installed: crate::mods::list_installed(state).map(|mods| {
            if mods
                .iter()
                .any(|installed| installed.project_id == record.project_id)
            {
                mods
            } else {
                let mut updated = mods;
                updated.push(record);
                updated
            }
        })?,
    })
}

async fn cleanup_legacy_external_mods(state: &AppState) -> AppResult<()> {
    let installed = crate::mods::list_installed(state)?;
    for incompatible_project in ["NNAgCjsB", "BpzUOKOJ", "xgpAkTGi", "w6x8nHjH"] {
        if installed
            .iter()
            .any(|item| item.project_id == incompatible_project)
        {
            crate::mods::remove_local_mod_now(state, incompatible_project)?;
        }
    }
    if let Ok(mut entries) = tokio::fs::read_dir(&state.paths.mods).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let file_name = entry.file_name().to_string_lossy().to_lowercase();
            if file_name.contains("perspectivemodredux") || file_name.contains("polynametag") {
                let _ = tokio::fs::remove_file(entry.path()).await;
            }
        }
    }
    Ok(())
}

async fn import_hitdelayfix(state: &AppState) -> AppResult<()> {
    let staging_path = state.paths.staging.join("HitDelayFix-1.0.1.jar");
    let expectation = crate::network::DownloadExpectation {
        maximum_size: 10 * 1024 * 1024,
        expected_size: Some(1_868_061),
        sha512: Some("b8b49155b836caf4e9c9ba03f803900fa3e3d9d45f96fa8487b04cd37ceacbb8b8c8794348c49be0a5d7d52e0dde12265c18db4292a9421f76f1cdb0e16ca0c2".to_string()),
        sha1: Some("cff27d11fb76527ccbf4b3cb74f9973ee5ac329d".to_string()),
    };
    let url =
        "https://github.com/ghast/HitDelayFixMod/releases/download/1.0.1/HitDelayFix-1.0.1.jar";
    let receipt = state
        .network
        .download(url, &staging_path, &expectation)
        .await?;
    validate_jar(&staging_path, 10 * 1024 * 1024)?;
    install_external_mod(
        state,
        &staging_path,
        "external-hitdelayfix",
        "hitdelayfix-1.0.1",
        "HitDelayFix",
        "ghast",
        "Original HitDelayFix mod downloaded from the author's GitHub release.",
        "1.0.1",
        "MIT",
        ModSource::Github,
        ModTrust::Verified,
        receipt.size,
        receipt.sha512,
    )?;
    tokio::fs::remove_file(&staging_path)
        .await
        .map_err(|error| AppError::io("Could not clean the HitDelayFix staging file", error))?;
    Ok(())
}

async fn import_animations(state: &AppState) -> AppResult<()> {
    let staging_path = state
        .paths
        .staging
        .join("OverflowAnimations-1.8.9-forge-2.2.2.jar");
    let expectation = crate::network::DownloadExpectation {
        maximum_size: 10 * 1024 * 1024,
        expected_size: Some(164_123),
        sha512: Some("b1167b5bd8207af1b95c755124d298d4c0ddc25e975da5e5eb9548d8e4336a00c878f75f999a1babd9e9cfc5b362352737651c266fcaa950815a513af42c0c5b".to_string()),
        sha1: Some("a982cba3ad482ca063fdd7d7852118f7380efc6e".to_string()),
    };
    let url = "https://cdn.modrinth.com/data/4Hfmgaef/versions/x99qPdUO/OverflowAnimations-1.8.9-forge-2.2.2.jar";
    let receipt = state
        .network
        .download(url, &staging_path, &expectation)
        .await?;
    validate_jar(&staging_path, 10 * 1024 * 1024)?;
    install_external_mod(
        state,
        &staging_path,
        "4Hfmgaef",
        "x99qPdUO",
        "Animatium Legacy (OverflowAnimations)",
        "Polyfrost",
        "Original 1.7 and modern animations mod for Minecraft 1.8.9.",
        "2.2.2",
        "LGPL-3.0-only",
        ModSource::Modrinth,
        ModTrust::FromModrinth,
        receipt.size,
        receipt.sha512,
    )?;
    tokio::fs::remove_file(&staging_path)
        .await
        .map_err(|error| AppError::io("Could not clean the animations staging file", error))?;
    Ok(())
}

async fn import_fullbright(state: &AppState) -> AppResult<()> {
    let staging_path = state.paths.staging.join("Fullbright-1.0.0.jar");
    let expectation = crate::network::DownloadExpectation {
        maximum_size: 10 * 1024 * 1024,
        expected_size: Some(20_086),
        sha512: Some("f9a54aeb27196958b75bb77d5025fa0c64f61eb605d86726354ba741d817769be9a5e287db578b103d2c7fb104d851fb16a3c46a378a3954ba05c6a48411ac25".to_string()),
        sha1: Some("05d70a7c20b0c974e557c9766b9de3fd8fc0bb63".to_string()),
    };
    let url = "https://cdn.modrinth.com/data/8L5i5hyX/versions/vHfx3jg3/Fullbright-1.0.0.jar";
    let receipt = state
        .network
        .download(url, &staging_path, &expectation)
        .await?;
    validate_jar(&staging_path, 10 * 1024 * 1024)?;
    install_external_mod(
        state,
        &staging_path,
        "8L5i5hyX",
        "vHfx3jg3",
        "Fullbright",
        "Modrinth author",
        "External Fullbright mod downloaded from its pinned Modrinth version.",
        "1.0.0",
        "See Modrinth project",
        ModSource::Modrinth,
        ModTrust::FromModrinth,
        receipt.size,
        receipt.sha512,
    )?;
    tokio::fs::remove_file(&staging_path)
        .await
        .map_err(|error| AppError::io("Could not clean the Fullbright staging file", error))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn install_external_mod(
    state: &AppState,
    source: &Path,
    project_id: &str,
    version_id: &str,
    name: &str,
    author: &str,
    description: &str,
    version: &str,
    license: &str,
    provider: ModSource,
    trust: ModTrust,
    file_size: u64,
    sha512: String,
) -> AppResult<()> {
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::InvalidInput,
                "External mod has no safe file name",
            )
        })?
        .to_owned();
    let destination = state.paths.mods.join(&file_name);
    atomic_copy(source, &destination)?;
    let record = InstalledMod {
        id: project_id.to_owned(),
        project_id: project_id.to_owned(),
        version_id: version_id.to_owned(),
        name: name.to_owned(),
        author: author.to_owned(),
        description: description.to_owned(),
        icon_url: None,
        version: version.to_owned(),
        release_type: ReleaseType::Release,
        downloads: 0,
        updated_at: Utc::now().to_rfc3339(),
        minecraft_version: crate::minecraft::MINECRAFT_VERSION.to_owned(),
        loader: "forge".to_owned(),
        environment: ModEnvironment::Client,
        license: license.to_owned(),
        file_size,
        dependency_count: 0,
        trust,
        compatibility: ModCompatibility::Compatible,
        compatibility_reason: None,
        installed: true,
        installed_version: Some(version.to_owned()),
        update_available: false,
        file_name,
        sha512,
        provider,
        required: false,
        installed_at: Utc::now().to_rfc3339(),
        dependencies: Vec::new(),
        dependents: Vec::new(),
    };
    if let Err(error) = crate::mods::register_local_mod(state, record) {
        let _ = fs::remove_file(destination);
        return Err(error);
    }
    Ok(())
}

pub async fn import(
    state: &AppState,
    request: ImportOptifineRequest,
) -> AppResult<ModOperationResult> {
    // The liveness check must happen under the operation lock: `launcher::launch`
    // holds that lock for the whole launch, so a check made before acquiring it
    // can pass and then rewrite `mods/` underneath a JVM that started meanwhile.
    {
        let _guard = state.operation_lock.lock().await;
        if !state.is_game_running() {
            let record = import_now(state, Path::new(&request.source_path))?;
            install_external_components(state).await?;
            return Ok(ModOperationResult {
                operation_id: Uuid::new_v4().to_string(),
                queued: false,
                installed: crate::mods::list_installed(state).map(|mods| {
                    if mods
                        .iter()
                        .any(|installed| installed.project_id == record.project_id)
                    {
                        mods
                    } else {
                        let mut updated = mods;
                        updated.push(record);
                        updated
                    }
                })?,
            });
        }
    }
    // The guard is released before enqueueing because `enqueue_optifine`
    // acquires the same non-reentrant lock.
    crate::mods::enqueue_optifine(state, request.source_path).await
}

pub(crate) async fn install_external_components(state: &AppState) -> AppResult<()> {
    import_hitdelayfix(state).await?;
    import_animations(state).await?;
    import_fullbright(state).await?;
    import_curated_modrinth_components(state).await?;
    cleanup_legacy_external_mods(state).await?;
    Ok(())
}

/// Pinned Private Pack components, as
/// `(project, version, name, author, version_number, license, file, size, sha1, sha512)`.
///
/// Every entry is pinned to an exact Modrinth version and both digests; nothing
/// here may use a floating "latest" version. Adding an entry also requires
/// adding its project id to `REMOVABLE_PACK_COMPONENTS` in `commands.rs`, which
/// `pack_components_are_all_removable` enforces.
type PackComponent = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    u64,
    &'static str,
    &'static str,
);

pub(crate) const CURATED_PACK_COMPONENTS: &[PackComponent] = &[
    ("jupr7Bf5", "MqLKfrk2", "FoamFix", "asiekierka", "0.6.3a", "Custom", "foamfix-0.6.3a-anarchy-1.8.x.jar", 86_043, "58f4f9165f1c91222e33493175c805879e75db6b", "d66854ac4680e7b7677a8cc43588dbe01d29724a36c66cbfa0808a110f6d96377918e831606a765358cd7d6ead0b7363bd0c62cb3b0d7941697d328dee9fc7be"),
        ("5uJtFIcj", "RLkvK4Y6", "No Hurt Cam", "Modrinth author", "1.0.0", "MIT", "nohurtcam-1.0.0.jar", 969_043, "b0061fc03cd59ccbeef45f3366811834e8455e0d", "c78d4f1a5de94b1b484279f8e33fb7a17c2d5f401d9cf93490cad2f3d7217dcb568de3792e5f60fe04f27f81dec9f5a9703fd9a0788b06d38a74a65a03bb9908"),
        ("YknNc5nN", "o16JHhlj", "PolyPatcher", "Polyfrost", "1.10.3", "CC-BY-NC-SA-4.0", "PolyPatcher-1.8.9-forge-1.10.3.jar", 27_195_510, "6f5198af6a4d39e27959cd7468c81e87a7f9889d", "8ef302645814a3dc3f06463ab92ddb331f3bdc3d388ad928cb2a5b37256af6815abc90e1bf28c0185cfafac7a74ede7f3f2a469754c786d93a6c12b9e7603293"),
        ("oCBQFmrZ", "iECFoMt9", "Phosphor Legacy Forge", "HowardZHY", "7", "GPL-3.0-or-later", "phosphor-universal.jar", 1_050_399, "f9329954e829f98c1fdb6762f930b33a63ba0690", "18f4437026b29127e4439b92b27eeb4d596bdb44bfd37e358c213ec9b7ebd26705f138b2e475546f02f2127f0ee4a4807c4e3356a86df9015df0b4cd55a0aa73"),
        ("r4AQF5mj", "UezAlzL0", "Velox Caelo", "Modrinth author", "1.1.0", "All Rights Reserved", "veloxcaelo-1.1.0.jar", 1_414_298, "694962ebb4f354c91cda5a62d888c382d9e2d095", "3580186e83bcfbc0f08879260ca71da43e47f1ff2dda44065e100d87679cfe4eb9c62fba01fbfe385f2d4c8902132698176d19b5e05e0040b0abb0c8f13c6858"),
        // Crash resilience: wraps the game loop so a crash shows a recoverable
        // screen instead of closing Minecraft (the 1.8.9 port of VanillaFix).
        ("nZ3E8WQz", "qDhse9AS", "CrashPatch", "Polyfrost", "2.0.2", "GPL-3.0-with-Minecraft-linking-exception", "CrashPatch-1.8.9-forge-2.0.2.jar", 580_078, "23f13fbf1f81133ed5bf0ad5278d8c51bac9a3d7", "6ddba5eb1184ceab16b70f7bb951115eb1b43717535fa4f765552c3729b6357b257056533ee2b17384b7ffe93f767e5cb2b4f036f3c977c174dadb302d08fee5"),
        // Direct mouse input for consistent aim: bypasses the OS pointer
        // acceleration that vanilla 1.8.9 inherits. Ships the OneConfig loader
        // already used by PolyPatcher. The '+' in the file name is a valid CDN
        // path segment and is verified to resolve to this exact artifact.
        ("tNZqMcok", "JRPGyssd", "Raw Input", "Chromatic", "0.1.8", "GPL-3.0-only", "RawInput-0.1.8+1.8.9-forge.jar", 54_000, "5ec927f2db69771f17a2097f89901560e65f4a6e", "3edd010f7d464d8ef59c435a124a0eb7e10745f4f7ed9d9ad06b8104e5279c8ea750cd2bca0e6bcaad3b34d5167c73347f41b14e5c293a47d0055d6ac84d99d0"),
        // Network fix: the vanilla multiplayer list reuses a fixed buffer, which
        // makes server pings load slowly or hang forever on larger lists.
        ("TdLuRq7y", "rDs3N6GK", "ServerlistBufferFixer", "Nixuge", "1.0.1", "Unlicense", "serverlistbufferfixer-1.0.1.jar", 986_621, "1b31178ce87c4a337e41b224797e06b7d5122ddc", "6430a6f15b695bb79ff7edfd3c34303b82be82bc697b542c89f703bdc498a444658fb3a59adadfd815e2c1b64cc6c06fc154111cb314e75de2df03801c42beba"),
        // Quality of life: makes the window closable during the Forge loading
        // screen, backporting the behaviour newer Forge versions already have.
        ("uhBpdFWZ", "VIjIWe6z", "QuickQuit", "Microcontrollers", "1.0.1", "LGPL-3.0-only", "QuickQuit-1.0.1.jar", 1_027_670, "fda7e2699d080d2d77810da31fcd506e4d03a447", "cf648b7f48364a79ec1086bb6fe9e35a8b6c00232b9b9a4551d85c59344af5e0a8fbafbb2489b4df7fa1b00e1185a2f1bc557e584dfbac6a6ccdd192124e23f1"),
];

async fn import_curated_modrinth_components(state: &AppState) -> AppResult<()> {
    for (project_id, version_id, name, author, version, license, file_name, size, sha1, sha512) in
        CURATED_PACK_COMPONENTS.iter().copied()
    {
        let url_version = version_id;
        let url = format!(
            "https://cdn.modrinth.com/data/{project_id}/versions/{url_version}/{file_name}"
        );
        let staging_path = state.paths.staging.join(file_name);
        if staging_path.exists() {
            tokio::fs::remove_file(&staging_path)
                .await
                .map_err(|error| AppError::io("Could not reset component staging", error))?;
        }
        let receipt = state
            .network
            .download(
                &url,
                &staging_path,
                &crate::network::DownloadExpectation {
                    maximum_size: 64 * 1024 * 1024,
                    expected_size: Some(size),
                    sha512: Some(sha512.to_owned()),
                    sha1: Some(sha1.to_owned()),
                },
            )
            .await?;
        validate_jar(&staging_path, 64 * 1024 * 1024)?;
        install_external_mod(
            state,
            &staging_path,
            project_id,
            version_id,
            name,
            author,
            "Pinned external component of Private Pack.",
            version,
            license,
            ModSource::Modrinth,
            ModTrust::FromModrinth,
            receipt.size,
            receipt.sha512,
        )?;
        tokio::fs::remove_file(&staging_path)
            .await
            .map_err(|error| AppError::io("Could not clean component staging", error))?;
    }
    Ok(())
}

pub(crate) fn import_now(state: &AppState, source: &Path) -> AppResult<InstalledMod> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| AppError::io("Could not inspect the selected OptiFine JAR", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::new(
            AppErrorCode::JarValidationFailed,
            "OptiFine must be imported from a regular local file",
        ));
    }
    validate_optifine_name(source)?;
    validate_jar(source, OPTIFINE_LIMIT)?;
    validate_optifine_contents(source)?;
    let (sha512, _, _) = hash_file(source)?;
    let suffix = sha512.get(0..12).map(str::to_owned).ok_or_else(|| {
        AppError::new(
            AppErrorCode::JarValidationFailed,
            "Could not derive a safe OptiFine file name",
        )
    })?;
    let file_name = format!("OptiFine-local-{suffix}.jar");
    let destination = state.paths.mods.join(&file_name);
    let old_record = crate::mods::list_installed(state)?
        .into_iter()
        .find(|installed| installed.project_id == "local-optifine");
    let backup = state
        .paths
        .staging
        .join(format!("optifine-{}.bak", Uuid::new_v4()));
    if let Some(previous) = &old_record {
        let old_path = state.paths.mods.join(&previous.file_name);
        if old_path.is_file() && old_path != destination {
            fs::rename(&old_path, &backup)
                .map_err(|error| AppError::io("Could not back up the previous OptiFine", error))?;
        }
    }
    let result = (|| -> AppResult<InstalledMod> {
        atomic_copy(source, &destination)?;
        let record = InstalledMod {
            id: "optifine-local".to_owned(),
            project_id: "local-optifine".to_owned(),
            version_id: suffix,
            name: "OptiFine 1.8.9".to_owned(),
            author: "sp614x".to_owned(),
            description: "OptiFine component of Private Pack. External HitDelayFix, Animatium Legacy and Fullbright are installed as separate verified records.".to_owned(),
            icon_url: None,
            version: "local import".to_owned(),
            release_type: ReleaseType::Release,
            downloads: 0,
            updated_at: Utc::now().to_rfc3339(),
            minecraft_version: crate::minecraft::MINECRAFT_VERSION.to_owned(),
            loader: "forge".to_owned(),
            environment: ModEnvironment::Client,
            license: "External local file".to_owned(),
            file_size: source
                .metadata()
                .map(|metadata| metadata.len())
                .unwrap_or(0),
            dependency_count: 0,
            trust: ModTrust::Verified,
            compatibility: ModCompatibility::Compatible,
            compatibility_reason: None,
            installed: true,
            installed_version: Some("local import".to_owned()),
            update_available: false,
            file_name,
            sha512,
            provider: ModSource::LocalImport,
            required: false,
            installed_at: Utc::now().to_rfc3339(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
        };
        crate::mods::register_local_mod(state, record.clone())?;
        Ok(record)
    })();
    match result {
        Ok(record) => {
            if backup.exists() {
                let _ = fs::remove_file(backup);
            }
            Ok(record)
        }
        Err(error) => {
            let _ = fs::remove_file(destination);
            if let Some(previous) = old_record {
                let previous_path = state.paths.mods.join(previous.file_name);
                if backup.exists() {
                    let _ = fs::rename(backup, previous_path);
                }
            }
            Err(error)
        }
    }
}

fn validate_optifine_name(path: &Path) -> AppResult<()> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.contains("optifine") && name.contains("1.8.9") && name.ends_with(".jar") {
        Ok(())
    } else {
        Err(AppError::new(
            AppErrorCode::ModIncompatible,
            "The selected file is not named as an OptiFine build for Minecraft 1.8.9",
        ))
    }
}

fn validate_optifine_contents(path: &Path) -> AppResult<()> {
    let file =
        File::open(path).map_err(|error| AppError::io("Could not open the OptiFine JAR", error))?;
    let mut archive = ZipArchive::new(file)?;
    let signatures = [
        "optifine/OptiFineClassTransformer.class",
        "optifine/OptiFineForgeTweaker.class",
        "Config.class",
    ];
    let found = signatures
        .iter()
        .filter(|entry| archive.by_name(entry).is_ok())
        .count();
    if found >= 2 {
        Ok(())
    } else {
        Err(AppError::new(
            AppErrorCode::JarValidationFailed,
            "The selected JAR does not contain expected OptiFine 1.8.9 classes",
        ))
    }
}

#[allow(dead_code)]
fn extract_version_from_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("OptiFine_1.8.9")
        .chars()
        .take(80)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{validate_optifine_name, CURATED_PACK_COMPONENTS};
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn requires_explicit_1_8_9_optifine_name() {
        assert!(validate_optifine_name(Path::new("OptiFine_1.8.9_HD_U_M5.jar")).is_ok());
        assert!(validate_optifine_name(Path::new("OptiFine_1.20.jar")).is_err());
        assert!(validate_optifine_name(Path::new("random-1.8.9.jar")).is_err());
    }

    #[test]
    fn every_pack_component_is_fully_pinned() {
        for (project, version, name, _author, _number, license, file, size, sha1, sha512) in
            CURATED_PACK_COMPONENTS.iter().copied()
        {
            assert!(!project.is_empty(), "{name} has no project id");
            assert!(!version.is_empty(), "{name} has no pinned version id");
            assert!(!license.is_empty(), "{name} has no license");
            assert!(file.ends_with(".jar"), "{name} is not a jar: {file}");
            assert!(size > 0, "{name} has no expected size");
            // A floating version would defeat the pin entirely.
            assert!(
                !version.eq_ignore_ascii_case("latest"),
                "{name} uses an unbounded version"
            );
            assert_eq!(sha1.len(), 40, "{name} has a malformed sha1");
            assert_eq!(sha512.len(), 128, "{name} has a malformed sha512");
            assert!(
                sha1.chars().all(|c| c.is_ascii_hexdigit())
                    && sha512.chars().all(|c| c.is_ascii_hexdigit()),
                "{name} has a non-hex digest"
            );
        }
    }

    #[test]
    fn pack_component_pins_are_unique() {
        let mut projects = HashSet::new();
        let mut files = HashSet::new();
        for (project, _v, name, _a, _n, _l, file, _s, _sha1, _sha512) in
            CURATED_PACK_COMPONENTS.iter().copied()
        {
            assert!(projects.insert(project), "{name} is pinned twice");
            // Two components sharing a staging file name would clobber each other.
            assert!(
                files.insert(file),
                "{file} is used by more than one component"
            );
        }
    }

    #[test]
    fn pack_components_are_all_removable() {
        // Uninstalling Private Pack must not strand a component on disk.
        let removable: HashSet<&str> = crate::commands::REMOVABLE_PACK_COMPONENTS
            .iter()
            .copied()
            .collect();
        for (project, _v, name, _a, _n, _l, _f, _s, _sha1, _sha512) in
            CURATED_PACK_COMPONENTS.iter().copied()
        {
            assert!(
                removable.contains(project),
                "{name} ({project}) is installed by the pack but never removed by it"
            );
        }
    }

    #[test]
    fn pack_components_are_folded_into_the_single_pack_card() {
        // Every pinned component belongs to Private Pack, so none of them may
        // show up as its own card in the installed list.
        for (project, _v, name, _a, _n, _l, _f, _s, _sha1, _sha512) in
            CURATED_PACK_COMPONENTS.iter().copied()
        {
            assert!(
                crate::commands::is_folded_pack_component(project),
                "{name} ({project}) is a pack component but is still listed separately"
            );
        }
        // The account switcher is a required standalone mod, not pack content.
        assert!(!crate::commands::is_folded_pack_component("cudtvDnd"));
        // The pack card itself is built from the local OptiFine record.
        assert!(!crate::commands::is_folded_pack_component("local-optifine"));
    }

    #[test]
    fn known_incompatible_projects_are_never_installed() {
        // These replace the chunk renderer or entity culling that OptiFine
        // already provides; `cleanup_legacy_external_mods` actively deletes them.
        for incompatible in ["NNAgCjsB", "BpzUOKOJ", "xgpAkTGi", "w6x8nHjH"] {
            assert!(
                !CURATED_PACK_COMPONENTS
                    .iter()
                    .any(|component| component.0 == incompatible),
                "{incompatible} is known to be incompatible but is pinned for install"
            );
        }
    }
}
