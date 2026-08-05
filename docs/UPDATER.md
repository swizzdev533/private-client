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

## Channels

Two side-by-side applications, never one app with a switch. A channel switch
would be a trap: the downgrade protection above refuses anything older, so
moving from beta back to stable would require a manual uninstall.

| | Stable | Beta |
|---|---|---|
| Product name | Private Client | Private Client Beta |
| Identifier | `com.privateclient.launcher` | `com.privateclient.launcher.beta` |
| Data root | `%LOCALAPPDATA%\Private Client` | `%LOCALAPPDATA%\Private Client Beta` |
| Endpoint | `releases/latest/download/latest.json` | `releases/download/beta/beta.json` |
| Release kind | normal release | pre-release, fixed `beta` tag |
| Footer badge | none | `BETA` |

The channel is compiled in from `PRIVATE_CLIENT_CHANNEL`, not read from a
setting or a file, so a running launcher cannot be redirected at the other
channel's data. An unrecognized value fails the build instead of defaulting.

**Stable installations never see a beta build.** GitHub's `/releases/latest/`
deliberately skips pre-releases, and the beta lives permanently under the
`beta` tag. That is the whole isolation mechanism — there is no filtering in
the client to get wrong.

Separate data roots mean the beta downloads its own Minecraft, Forge, and mods
on first launch. That is the point: a broken beta cannot corrupt the instance
the stable client depends on.

### Workflow

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.private-client\keys\private-client-updater.key" -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '...'

pnpm release:beta        # build + publish the private beta
pnpm release:promote     # beta becomes the public release, next cycle opens
```

`release:promote` does not reuse the beta installer. That artifact carries the
beta product name, identifier, and data directory, so publishing it as stable
would install a second "Private Client Beta" rather than upgrading anyone. It
rebuilds the same commit on the stable channel instead, which is why it refuses
to run against a dirty working tree — the promoted build must be the code that
was tested.

After promotion the version is bumped (minor by default, `-NextVersion` to
override) and committed, so the next beta cycle starts on a version strictly
greater than the published one.

> Both channels bundle into the same directory. After `pnpm build:beta`, the
> binary at `target\...\release\private-client.exe` is the **beta** build, so
> running `verify:release` or `package-release` by hand at that point inspects
> the beta. The `release:beta` and `release:promote` scripts always rebuild the
> channel they publish, so this only bites manual invocations.

## Scope

Core and required-mod content use the pinned exact-version/hash model in
`docs/MOD_SYSTEM.md`. They are separate components and are not delivered
through this launcher updater.
