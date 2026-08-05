# Building

## Supported release host

The checked release target is Windows 11 x64. Windows 10/11 users need the
Evergreen Microsoft Edge WebView2 Runtime. The build host needs:

- Node.js 24 or newer and pnpm 11.18.0;
- Rust stable and Cargo;
- the `x86_64-pc-windows-msvc` Rust target;
- either the native MSVC toolchain plus Microsoft C++ Build Tools, or
  `cargo-xwin` 0.23 with clang, `llvm-lib`, and `llvm-rc`;
- Java 8 (Temurin 8 is tested);
- network access to package registries and official legacy Forge repositories.

No application secret is required. Code signing is optional for a local build but
mandatory before a trusted public release.

## One-command build

```powershell
corepack enable
pnpm install --frozen-lockfile
pnpm build
```

`scripts/build/build-all.ps1` runs UI checks, Rust checks/tests, the Core build,
the production Tauri NSIS build, copies the raw app and installer to
`artifacts\release`, names them `PrivateClient.exe` and `setup.exe`, and writes
`SHA512SUMS.txt`. Release binaries are always compiled for
`x86_64-pc-windows-msvc`.

The launcher build prefers a native MSVC host. Select it on a normal Windows
release workstation:

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

If Visual Studio Build Tools are unavailable, install the MSVC target and the
cross-build tools. The build script then selects the fallback automatically:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin --version 0.23.0 --locked
$env:XWIN_CROSS_COMPILER = 'clang'
pnpm build:launcher:xwin
```

`cargo-xwin` still produces an MSVC ABI binary. A GNU binary from
`target\release` is never accepted as a standalone release because its WebView2
loader linkage can require a sidecar DLL. `pnpm verify:release` parses the PE
import directory directly and rejects a non-x64 launcher or an import of
`WebView2Loader.dll`. The produced binaries must also be smoke-tested on a clean
Windows VM.

## Individual checks

```powershell
pnpm lint
pnpm typecheck
pnpm test
pnpm test:rust
pnpm test:core
pnpm --dir apps/launcher build
pnpm build:launcher
pnpm verify:release
```

## Outputs

- Raw app:
  `apps\launcher\src-tauri\target\x86_64-pc-windows-msvc\release\private-client.exe`
- Tauri NSIS:
  `apps\launcher\src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\*-setup.exe`
- Curated release: `artifacts\release\PrivateClient.exe`
- Curated installer: `artifacts\release\setup.exe`
- Checksums: `artifacts\release\SHA512SUMS.txt`

The installer is per-user and does not require administrator access. It includes
the launcher and original assets only—not Minecraft, Forge, OptiFine, account
material, or third-party mod JARs.
