# Private Client Beta 1.0

Private Client is a Windows launcher for an isolated Minecraft Java Edition
1.8.9 + Forge 11.15.1.2318 instance. It has exactly two primary views:
**PLAY** and **MODS**.

"Beta 1.0" is the product line, not the build number. The build version is
shown in the launcher footer and is what the signed updater compares; see
`CHANGELOG.md` for released builds.

The installer contains only original Private Client code and artwork. Minecraft,
Forge metadata, libraries, assets, and user-selected mods are fetched from their
official sources after installation. OptiFine is never bundled. Private Pack may
download the exact official 1.8.9 HD U M5 file and accepts it only when its
pinned size, SHA-1, SHA-512, and JAR structure match.

> Private Client is not affiliated with Mojang Studios or Microsoft.

## What works

- isolated per-user data under `%LOCALAPPDATA%\Private Client`;
- Java 8 discovery and compatibility checks;
- Minecraft 1.8.9 metadata/file preparation and pinned Forge installer flow;
- explicit launch state machine, single-game lock, local logs, crash summary;
- live Modrinth search restricted to Minecraft 1.8.9 + Forge;
- pinned-version selection, dependency plans, hash verification, JAR validation,
  atomic install/remove/update records, and queued operations while the game runs;
- automatic hash-pinned OptiFine installation with a local-import fallback;
- token-free profile bridge written by Private Client Core;
- PrivetBadge association (nametag + TAB) via pinned presence API;
- local diagnostics and log export;
- reduced-motion, DPI-aware monochrome launcher UI;
- raw Windows application EXE and NSIS `setup.exe`.

## Build

Prerequisites: Windows 10/11 x64, WebView2, Node.js, pnpm, Rust, and Java 8.

```powershell
pnpm install
pnpm build
```

Release artifacts are copied to `artifacts\release`. See
[`docs/BUILDING.md`](docs/BUILDING.md) for exact commands and toolchain notes.

## Privacy and account boundary

The launcher never asks for an email, password, 2FA code, Microsoft token, Xbox
token, or cookie. Account authentication is delegated to a pinned, external
in-game authentication mod. Private Client Core observes only the sanitized
Minecraft session outcome and writes username, UUID, skin model/path, timestamp,
and schema version. It does not copy session tokens.

External services necessarily see ordinary connection data such as the source IP
when the user requests their files. PrivetBadge uses a short-lived association
presence API (UUID/username + hashed server identity only; no session tokens).
There is no analytics/telemetry product. See [`docs/PRIVACY.md`](docs/PRIVACY.md)
and [`docs/NETWORK_POLICY.md`](docs/NETWORK_POLICY.md).

## Repository

- `apps/launcher` — React/TypeScript UI and Tauri/Rust backend;
- `apps/association-api` — Vercel + Supabase presence API for PrivetBadge;
- `minecraft/private-client-core` — Java 8 Forge 1.8.9 mod;
- `manifests` — versioned client/mod/runtime metadata and JSON schemas;
- `packages/contracts` — shared wire-format documentation;
- `fixtures` — deterministic API/profile fixtures for tests;
- `scripts` — build, release, and verification tooling;
- `docs` — architecture, security, privacy, operations, and release guides.

No Minecraft, OptiFine, account secrets, or third-party mod JAR is stored here.
