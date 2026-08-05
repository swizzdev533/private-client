# Updater

The launcher updates itself over a signed channel served from GitHub Releases.
Update traffic is performed by `tauri-plugin-updater` in Rust; the frontend
never fetches, verifies, or writes an update artifact.

## Trust boundary

| Property | Enforcement |
|---|---|
| Manifest authenticity | minisign signature over the artifact, verified against the public key pinned in `tauri.conf.json` |
| Artifact integrity | plugin verifies the `.sig` before the installer is written or executed |
| Transport | HTTPS only, to the single pinned endpoint; an unreachable host means "updates unavailable", never a fallback URL |
| Downgrade / replay | `updater::check` parses both versions as strict `major.minor.patch` and requires the advertised version to be strictly greater |
| Unbounded versions | `Version::parse` rejects `latest`, `v1.2.3`, `1.0`, and prerelease tags, so a tag can never satisfy a comparison |
| Running game | `updater::install` takes the operation lock and refuses while Minecraft is active |
| Release notes | attacker-influenced text: control characters stripped, bounded to 2000 characters in Rust and re-validated by the frontend schema |
| Consent | nothing installs without an explicit user action; the `autoUpdateChecks` setting only gates the background *check* |

## Configuration

`apps/launcher/src-tauri/tauri.conf.json`:

- endpoint: `https://github.com/swizzdev533/private-client/releases/latest/download/latest.json`
- `pubkey`: the minisign public key, safe to commit
- `bundle.createUpdaterArtifacts: true`: makes the build emit `<installer>.sig`

The corresponding **private** key lives outside the repository at
`%USERPROFILE%\.private-client\keys\private-client-updater.key`. It must never
be committed, logged, or attached to a release.

The key is passphrase-protected. Both the key file and its passphrase are
required to sign a release; back them up offline and separately. Losing either
means no existing installation can ever be updated again.

Regenerating the key (`tauri signer generate -f`) produces a new public key that
must be pinned in `tauri.conf.json`. Doing that after a public release breaks
updates for every installation already in the field, so treat the current key as
permanent from the first published release onward.

## Release procedure

1. Bump `version` in `apps/launcher/src-tauri/tauri.conf.json`, both
   `package.json` files, and `Cargo.toml`; add a `CHANGELOG.md` entry.
2. Export the signing key for the build:
   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.private-client\keys\private-client-updater.key" -Raw
   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Read-Host 'Key passphrase' -AsSecureString |
       ForEach-Object { [Runtime.InteropServices.Marshal]::PtrToStringAuto(
           [Runtime.InteropServices.Marshal]::SecureStringToBSTR($_)) }
   ```
   Prompt for the passphrase rather than pasting it into a script or a command
   line, where it would persist in shell history and process listings.
3. `pnpm build` — produces the installer and its `.sig`.
4. `pnpm verify:release` — PE and hash verification.
5. Authenticode-sign the installer, then **rebuild the signature**: the `.sig`
   must cover the signed bytes, or the updater will reject the artifact.
6. `pnpm release:manifest -Tag v<version>` — writes `artifacts/release/latest.json`.
7. Publish a GitHub release tagged `v<version>` with three assets: the
   installer, its `.sig`, and `latest.json`.

Step 5 is the ordering trap worth repeating: sign the EXE first, then generate
the updater signature over the final file.

## Verification before publishing

- `cargo test updater` covers downgrade, replay, unparsable versions, numeric
  ordering, and notes bounding.
- `pnpm test` covers the frontend gate: the background check runs only when
  `autoUpdateChecks` is on, stays silent on failure, and never installs
  without a user action.
- Externally unverified until run by the release owner: clean-VM install of
  version N followed by an in-place update to N+1, rollback on a corrupted
  artifact, and update refusal while the game is running.

## Scope

Core and required-mod content use the pinned exact-version/hash model in
`docs/MOD_SYSTEM.md`. They are separate components and are not delivered
through this launcher updater.
