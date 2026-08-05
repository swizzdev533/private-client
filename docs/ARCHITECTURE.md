# Architecture

## Product boundary

Private Client is a local desktop application. It has no service-side component,
account database, analytics collector, or advertising integration. The Windows
launcher owns installation and processes; Private Client Core owns in-game
session observation and the sanitized profile bridge. External providers remain
independent trust domains.

The first release targets Windows x64. Platform-specific path/process adapters
sit behind small Rust modules so Linux and macOS implementations can be added
without changing UI contracts or persisted formats.

## Components

```text
React UI
  │ typed invoke + bounded progress events
  ▼
Tauri command adapters
  ├─ application state / operation queue
  ├─ Java discovery
  ├─ Minecraft + Forge preparation
  ├─ process supervision + local logs
  ├─ Modrinth client + transactional mod store
  ├─ profile watcher
  ├─ hash-pinned OptiFine download and local import
  └─ diagnostics
       │
       ▼
%LOCALAPPDATA%\Private Client
  ├─ instance\game
  ├─ instance\mods
  ├─ profiles
  ├─ skins
  ├─ cache
  ├─ logs
  ├─ manifests
  ├─ staging
  ├─ backups
  └─ config

Minecraft 1.8.9 + Forge
  ├─ pinned external in-game account mod
  └─ Private Client Core
       ├─ atomic, token-free profile.json
       └─ PrivetBadge association (HTTPS presence + nametag/TAB)

Association API (Vercel + Supabase)
  └─ POST /api/v1/presence → peers on SHA-256(server)
```

Frontend code cannot read arbitrary files or spawn processes. Every Tauri
command accepts a closed typed input, validates it, delegates to a domain
service, and returns a serializable domain result or stable error code.

## Paths and isolation

Large game data uses `%LOCALAPPDATA%\Private Client` rather than roaming
`%APPDATA%`. This is an intentional divergence from the prompt's example path:
game assets and runtimes can occupy gigabytes and should not roam with a Windows
profile. No operation writes to the normal `.minecraft` directory.

`AppPaths` is the only source of domain paths. All user-influenced file names are
reduced to validated leaf names. Existing inputs are canonicalized, rejected if
they escape the allowed root, and rejected when a reparse point/symlink changes
the trust boundary. Downloads and installs occur in per-operation staging
directories on the same volume as their target.

## Launch flow

The backend owns a single explicit state machine:

```text
IDLE → VALIDATING → CHECKING_RUNTIME → PREPARING_INSTANCE
     → VERIFYING_GAME_FILES → INSTALLING_GAME_FILES
     → VERIFYING_FORGE → INSTALLING_FORGE
     → CHECKING_REQUIRED_MODS → APPLYING_PENDING_CHANGES
     → BUILDING_LAUNCH_COMMAND → LAUNCHING → RUNNING
     → STOPPING | EXITED | FAILED
```

Only one transition task and one game process may exist. A lock file records the
child PID and launch timestamp; stale locks are validated against the process
table before removal. Progress events contain a state, user-safe message,
optional percentage, and cancellation flag. Cancellation is honored only before
an atomic commit or child process start.

## Minecraft installation

1. Fetch Mojang's official version manifest over HTTPS.
2. Select the exact `1.8.9` entry and fetch its version JSON.
3. Validate bounded JSON and HTTPS artifact URLs.
4. Stream the client, libraries, logging config, asset index, and referenced
   assets to staging.
5. Verify provider SHA-1 and expected size where supplied.
6. Move files into the isolated standard launcher layout.
7. Re-verify the set on each launch and repair missing/corrupt entries only.

Minecraft content is not included in the repository or installer. A network
failure is surfaced as an offline/repair error; the launcher never claims the
instance is installed when verification is incomplete.

## Forge installation

Forge is pinned to `1.8.9-11.15.1.2318-1.8.9`. The launcher downloads the official
installer from Forge Maven, verifies its pinned SHA-1, and checks its bounded JAR
structure.

The legacy SimpleInstaller contained in this Forge release does **not** expose a
headless `--installClient` operation; that flag would be a fictional interface.
Its client path is Swing-only. Private Client therefore performs the same
data-driven installation without launching an interactive installer: it reads
the official `install_profile.json`/version metadata inside the verified
installer, validates every Maven coordinate and URL, extracts the official
universal JAR entry, downloads the exact libraries from approved hosts, and
writes the derived Forge version JSON into the isolated launcher layout.

After the transaction, the expected Forge version JSON, universal JAR, libraries,
and inheritance from Minecraft 1.8.9 are re-verified. The installer itself is
never bundled. If the legacy profile format is missing, changes unexpectedly, or
cannot be satisfied, Private Client stops with `ForgeInstallationFailed`; it
does not invent a version manifest, automate Swing, or use an untrusted mirror.

## Authentication decision

The launcher intentionally does not implement Microsoft OAuth and never receives
account tokens. A pinned external in-game account mod performs the official
flow inside Minecraft. Private Client Core reads only the resulting public
session identity required to gate premium multiplayer and expose a sanitized
profile. The bridge never serializes an access token or full session object.

This boundary means the external authentication mod must be reviewed whenever
its pinned version changes. The current legacy 1.8.9 line is external and no
longer actively supported; the launcher labels it accordingly. A real licensed
account test is required before calling authentication verified for a release.

## Profile bridge

`profiles/profile.json` is the source of truth. Core writes a bounded temporary
JSON document, syncs it, and atomically replaces the prior file. The launcher
watches the directory and revalidates the full document before updating UI.
Permitted fields are schema version, username, UUID, `classic|slim`, a path
inside the skin cache, and update time. See `PROFILE_BRIDGE.md`.

## Modrinth and mod transactions

Search uses official Modrinth endpoints with Forge and 1.8.9 facets. A project
becomes installable only after exact versions are fetched and filtered. Releases
are preferred to beta; alpha requires a separate explicit action. The chosen
version ID and file are pinned in the local record.

Required dependencies are resolved into an acyclic graph before network writes.
Each transaction:

1. validates the plan, target names, disk budget, hosts, and licenses;
2. downloads all files into a unique staging directory;
3. verifies provider hashes and computes local SHA-512;
4. parses the ZIP central directory and checks for a mod JAR structure;
5. backs up replaced managed files;
6. atomically moves the complete set;
7. atomically writes the installed database;
8. removes staging/backups after successful post-verification.

Any failure before commit deletes staging. Any failure during commit restores
the complete backup set and prior database. Mod operations requested while the
game runs enter a local queue.

## OptiFine

OptiFine is not bundled. Private Pack reads the official OptiFine landing page,
accepts only its narrowly validated M5 download token, constructs the fixed
official-host URL, and verifies pinned size, SHA-1, SHA-512, JAR structure, and
1.8.9 class signatures before installation. The native local-import path remains
available as a fallback.

## Updates

Launcher updates require a separately hosted, versioned manifest and a publisher
signing identity. Neither is invented in this repository. The local updater
parser, hash checks, backups, and rollback formats are prepared, but automatic
publishing is disabled. Core and required-mod manifests can be updated only when
the game is stopped and every file hash is verified.

## Error model

Rust domain failures map to stable codes (for example `JavaNotFound`,
`HashMismatch`, `UnsafeRedirect`, `DependencyCycle`, and `LaunchFailed`). The UI
maps each code to a title, explanation, recovery action, and local log location.
Raw stack traces, authorization values, query strings, and arbitrary provider
responses are never the primary message.

## Deliberate limitations

- Release artifacts are unsigned until the owner supplies a Windows code-signing
  certificate; SmartScreen may warn.
- A clean Windows VM, licensed Minecraft account, and real Microsoft sign-in are
  external release gates and cannot be replaced by fixture tests.
- The NSIS installer is online-capable, not an offline Minecraft distribution.
- The external 1.8.9 account mod is pinned but must undergo a fresh source and
  live-flow review before public distribution.
