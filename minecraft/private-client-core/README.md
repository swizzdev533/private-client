# Private Client Core

Private Client Core is a client-only Forge mod for Minecraft Java Edition
1.8.9. It targets Forge `11.15.1.2318` and Java 8.

The mod contains real Forge integration and a testable, Forge-independent
domain layer. It does not contain Minecraft, Forge, OptiFine, an account
switcher, access tokens, or any third-party mod binary.

## Included behavior

- `@Mod` bootstrap with deterministic startup, ready and shutdown lifecycle;
- lightweight event bus with listener isolation and unregister support;
- module API with dependency/conflict checks, activation rollback and a
  deny-policy for cheat capabilities;
- read-only FPS, coordinates and JVM-memory HUD modules;
- versioned Core configuration, v1-to-v2 migration, validation, atomic writes
  and quarantine of corrupt input;
- session observer that immediately reduces the Minecraft credential to a
  boolean and never places the token in a domain object, event, file or log;
- mandatory atomic file-based authenticated profile bridge;
- local log redaction for authorization headers, bearer values, cookies,
  session identifiers, passwords and OAuth-style token fields.

There are no combat, movement, packet-manipulation, anti-cheat bypass, aim,
reach, velocity, autoclick, ESP, fly, scaffold or similar modules.

## Session guard boundary

Singleplayer and integrated-server connections remain available without an
authenticated account.

For remote multiplayer, the guard requires all of the following local evidence:

1. a valid Minecraft username;
2. a non-nil UUID;
3. vanilla session type `MOJANG` (the type used by authenticated 1.8.9
   sessions, including a compatible Microsoft account switcher);
4. a non-placeholder credential;
5. a matching authenticated `GameProfile` identity.

The normal vanilla/Realms multiplayer entry screen is replaced before the user
can start a connection. A network-event fallback closes remote connections
started directly by another mod. The Core does not invent authentication,
validate passwords, contact Microsoft, or bypass the server session check; the
destination server remains the final authority.

The blocking screen tells the player to use the separately installed in-game
account switcher from the main menu. This repository intentionally does not
bundle or impersonate that project.

## Local files

The default shared data root is `%APPDATA%\PrivateClient`. Tests or managed
launches may override it with the trusted JVM property
`-Dprivateclient.dataDir=<absolute path>`.

- Core config: `%APPDATA%\PrivateClient\core\config.json`
- Profile bridge: `%APPDATA%\PrivateClient\profile\profile.json`

The profile JSON writer emits exactly:

```text
schemaVersion
username
uuid
skinModel
skinPath
updatedAt
```

`skinPath` is empty or the normalized relative path
`cache/profiles/<uuid>/skin.png`. Absolute paths, traversal and symbolic-link
targets are rejected. When the current session is not authenticated, the
published profile is removed so the launcher cannot display an offline name as
a premium identity.

## Reproducible build inputs

- JDK: Java 8
- Gradle wrapper distribution: `2.7`
- Gradle wrapper bootstrap SHA-256:
  `498495120a03b9a6ab5d155f5de3c8f0d986a449153702fb80fc80e134484f17`
- Gradle distribution SHA-256:
  `cde43b90945b5304c43ee36e58aab4cc6fb3a3d5f9bd9449bb1709a68371cb06`
- ForgeGradle:
  `2.1-20211118.174922-42`
- ForgeGradle SHA-256:
  `29f4f9a4b7ad917937d6ca761404ed4c56ee2a716cbfdd190b9aa99f25eb4695`
- Forge/Minecraft coordinate:
  `1.8.9-11.15.1.2318-1.8.9`
- MCP mappings: `stable_22`

ForgeGradle 2.1 is legacy and prints an upstream “unsupported version” warning.
That warning is expected. The pinned build is currently functional on the
specified JDK and produces a reobfuscated mod JAR.

Windows build:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-core.ps1
```

Or directly:

```powershell
$env:JAVA_HOME = 'C:\Program Files\Eclipse Adoptium\jdk-8.0.492.9-hotspot'
.\gradlew.bat --no-daemon clean build
```

The wrapper bootstrap was generated with Gradle 8.9, runs on Java 8, and only
downloads the pinned Gradle 2.7 build distribution. The script verifies the
bootstrap before execution, and the bootstrap verifies the distribution
SHA-256. The build file verifies ForgeGradle before applying its plugin.
ForgeGradle then downloads the pinned userdev and mappings from
`maven.minecraftforge.net`; `scripts/build-core.ps1` verifies their pinned
SHA-256 values after resolution.

Outputs:

- `build/libs/private-client-core-1.0.0.jar`
- `build/libs/private-client-core-1.0.0.jar.sha512`
- `build/reports/tests/index.html`

Do not copy anything from `.gradle`, the Gradle cache, `run`, or Forge userdev
into a release. Only the reobfuscated Core JAR and its checksum are release
artifacts.
