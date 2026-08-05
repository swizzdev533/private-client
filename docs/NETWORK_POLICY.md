# Network Policy

All production HTTP clients reject plain HTTP, userinfo in URLs, unknown ports,
IP-literal hosts, and redirects that leave the allowlist. Redirect count, body
size, connection/read timeout, and JSON depth/size are bounded.

| Host | Purpose | Downloaded | Sent | Auth | Cache/retention |
|---|---|---|---|---|---|
| `piston-meta.mojang.com` | Official Minecraft metadata | version manifests and JSON | standard HTTP metadata only | none | versioned local cache |
| `piston-data.mojang.com` | Official Minecraft client data | client JAR and metadata-addressed objects | standard HTTP metadata only | none | isolated instance |
| `launcher.mojang.com` | Legacy official 1.8.9 client/library host referenced by signed metadata | exact client/library objects | standard HTTP metadata only | none | isolated instance |
| `libraries.minecraft.net` | Minecraft libraries | exact Maven artifacts | standard HTTP metadata only | none | isolated instance |
| `resources.download.minecraft.net` | Minecraft assets | hash-addressed assets | standard HTTP metadata only | none | isolated instance |
| `launchermeta.mojang.com` | Legacy official metadata referenced by 1.8.9 | manifests/config | standard HTTP metadata only | none | versioned local cache |
| `files.minecraftforge.net` | Human/metadata Forge source | pinned Forge metadata/redirect | standard HTTP metadata only | none | short metadata cache |
| `maven.minecraftforge.net` | Official Forge Maven | pinned 1.8.9 installer/libraries | standard HTTP metadata only | none | isolated cache |
| `api.modrinth.com` | Official mod search/version API | public project/version/license metadata | query, paging, facets; no profile/mod list | none | bounded metadata cache |
| `cdn.modrinth.com` | Official Modrinth files/icons | selected JAR/icon from exact metadata URL | standard HTTP metadata only | none | installed file/icon cache |
| `sessionserver.mojang.com` | Public profile/skin lookup fallback | public profile properties | UUID only when user requested/profile cache misses | none | bounded skin cache |
| `textures.minecraft.net` | Official public skin texture | selected skin PNG | standard HTTP metadata only | none | bounded skin cache |
| `optifine.net` | Official OptiFine source | fixed 1.8.9 HD U M5 landing page and hash-pinned JAR | fixed file name and short-lived token only | none | managed mod installation |
| `login.microsoftonline.com` | In-game Microsoft authentication | authorization UI/tokens handled by external in-game mod | data defined by Microsoft flow; never routed through launcher | Microsoft, in game | launcher: none |
| `login.live.com` | Legacy endpoint potentially used by pinned in-game mod | authentication UI/flow | external mod-controlled; never routed through launcher | Microsoft, in game | launcher: none |
| `api.minecraftservices.com` | Minecraft entitlement/profile in in-game flow | account entitlement/profile | external mod-controlled | Microsoft bearer, in game | launcher: none |
| `private-client-association.vercel.app` | PrivetBadge association presence (Core only) | peer UUID list for current hashed server | `uuid`, `username`, SHA-256 `serverHash`, `clientVersion` (no tokens, no raw host) | none | Blob objects TTL ~45s by upload time |
| `github.com` / `objects.githubusercontent.com` / `release-assets.githubusercontent.com` | Pinned GitHub release assets (mods + managed Java) | exact release assets | standard HTTP metadata only | none | downloads cache + managed install |
| Managed Java pin `jdk8u442-b06` (via GitHub hosts above) | Auto-provision Temurin JRE 8 when none is installed | pinned ZIP `OpenJDK8U-jre_x64_windows_hotspot_8u442b06.zip` | standard HTTP metadata only | none | `%LOCALAPPDATA%\Private Client\runtime\java8-8u442-b06\` until replaced by a newer pin |

## Launcher updates

| Host | Purpose | Downloaded | Sent | Auth | Cache/retention |
|---|---|---|---|---|---|
| `github.com` / `objects.githubusercontent.com` / `release-assets.githubusercontent.com` | Signed launcher self-update | `latest.json` manifest and the exact NSIS installer named by it | standard HTTP metadata only; no version query, identifier, or profile data | none | staged installer removed after install |

Requests are made only by `tauri-plugin-updater` in Rust, never by the webview,
so no CSP `connect-src` entry is required. The manifest and the installer are
verified against the minisign public key pinned in `tauri.conf.json` before
anything is written or executed. An unreachable or unparsable endpoint means
“updates unavailable”, never a fallback to an arbitrary URL. A version that is
not strictly newer, or not strict `major.minor.patch`, is rejected. See
`UPDATER.md` for the full trust boundary.

A background check runs at startup only when the user enables
`autoUpdateChecks`; it is off by default. Installation always requires an
explicit user action and never runs while Minecraft is active.

Core and required-mod content are separate components and are not delivered
through the launcher updater.

The managed Java runtime is enabled for Windows x64 only, using the pinned
Temurin 8u442-b06 JRE ZIP above (exact URL, size, SHA-512/SHA-1). PLAY prefers
a compatible system or configured Java 8; otherwise it downloads that pin into
`runtime/`. Changing the pin requires updating the constants, network policy,
and trust-boundary tests together.

## Request hygiene

Launcher requests never add username, UUID, Microsoft/Xbox tokens, active server,
full installed-mod list, local filesystem path, device fingerprint, stable
installation identifier, or local logs. The user agent contains only product and
major version needed for responsible API use.

Private Client Core may call the pinned association API host above while the
player is on a remote multiplayer server with an authenticated session. That
request intentionally includes the public Minecraft UUID/username and a
SHA-256 of `host:port` so other Private Client users on the same server can
render PrivetBadge. Raw server hostnames, session tokens, and Microsoft
credentials are never sent. Redirects away from the pinned host are rejected.

No catch-all networking capability exists in the frontend. Only Rust adapters
perform launcher network I/O. Association HTTPS is performed only by Core with
a fixed allowlist.

Discord Rich Presence uses only the Discord desktop client's local IPC pipe. It
does not open an HTTP connection, authenticate a Discord account, or store a
Discord token. Presence contains fixed product/version text and a start time;
player and server data are excluded.

## In-game profile rendering

The in-game main menu never downloads a skin or resolves a profile over HTTP.
It renders only a launcher-validated PNG from the bounded local profile cache,
after checking the UUID-derived path, symlink status, file size, and image
dimensions. A missing or invalid cache entry falls back to Minecraft's local
default skin; account credentials and session tokens are never exposed to the
menu or to Core modules.
