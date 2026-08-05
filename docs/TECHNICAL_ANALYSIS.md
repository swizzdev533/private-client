# Technical analysis and decisions

Date: 2026-07-30

## Build workstation

- Windows 11 Pro x64
- Node.js 24.18.0, pnpm 11.18.0
- Rust/Cargo 1.97.1, active `x86_64-pc-windows-gnu`
- Temurin OpenJDK/Javac 8u492 x64
- Edge WebView2 Runtime 150.0.4078.105
- Tauri NSIS 3.11 and WiX 3.14 caches available
- no Git, MSVC Build Tools, Windows SDK, code-signing certificate, or signing key

## Decision: Tauri 2 + React

The requested stack is retained. WebView2 is installed, modern Rust exceeds the
minimum required by Tauri 2, and the UI can be compiled by Vite/TypeScript.
Tauri's NSIS target produces the requested setup executable while the Rust target
produces a separate raw application EXE.

Official Tauri support on Windows uses the MSVC target and Microsoft C++ Build
Tools. This workstation has LLVM/clang plus `cargo-xwin` 0.23, so the local
release is cross-built for the same `x86_64-pc-windows-msvc` ABI. GNU launcher
binaries are explicitly rejected because they import a sidecar
`WebView2Loader.dll`; the release verifier parses the PE import table and
requires a standalone MSVC x64 executable. A trusted public build must still be
repeated on a clean supported host and Authenticode-signed.

Sources:

- https://v2.tauri.app/start/prerequisites/
- https://v2.tauri.app/distribute/windows-installer/

## Decision: pinned legacy game stack

Minecraft is exactly `1.8.9`. Forge is pinned to official recommended/latest
legacy build `11.15.1.2318` and official installer SHA-1
`ec0293ff0776b8831f2ed90511bab76e635dda0c`. Java 8 is isolated from the modern
launcher toolchain.

The runtime launcher uses Mojang's official version manifest and Forge's official
Maven. Neither game nor Forge files are redistributed. Inspection of the pinned
SimpleInstaller 1.7.7 artifact confirmed that its CLI has `--installServer`,
`--extract`, and `--offline`, but no `--installClient`. The launcher therefore
uses the verified installer's own `install_profile.json`, version metadata, and
universal entry to reproduce its client file layout transactionally. It does not
invent a CLI flag or automate the Swing installer.

Sources:

- https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
- https://files.minecraftforge.net/net/minecraftforge/forge/index_1.8.9.html

## Decision: account boundary

No Microsoft OAuth code is implemented in the launcher. The exact legacy
In-Game Account Switcher Modrinth release `uI9n4nDb`
(`7.1.2-fo1.8.9`, LGPL-3.0-or-later) is pinned by provider hashes. It is
downloaded at first preparation and remains an external component. Core sees only
the post-authentication public Minecraft session identity.

The upstream project states that old 1.8.9 versions are no longer supported.
Therefore source review and a live licensed account test remain mandatory public
release gates. The launcher never reads, copies, logs, or backs up the external
mod's token storage.

Sources:

- https://modrinth.com/mod/in-game-account-switcher
- https://api.modrinth.com/v2/version/uI9n4nDb

## Decision: data root

Use `%LOCALAPPDATA%\Private Client`, not roaming `%APPDATA%`, because game assets
and libraries can be large. All data remains local and per-user. This is a
documented technical refinement of the prompt's example directory.

## Decision: updater disabled by default

No release host, signature public key, Authenticode certificate, or private
publisher identity was supplied. The release still contains manifest parsing,
hash verification, backup, and rollback boundaries, but network update discovery
is disabled until those inputs are intentionally configured and tested.

## Legal/verification boundaries

The installer can be complete as a launcher but cannot be a fully offline game
installer without violating the explicit redistribution constraints. A valid
Minecraft license, real Microsoft flow, clean Windows VM, publisher certificate,
and release hosting are external acceptance inputs. Tests use fixtures and do
not fake these outcomes.
