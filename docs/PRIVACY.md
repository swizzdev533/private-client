# Privacy

Private Client has no advertising SDK, no analytics product, and no Private
Client user account database beyond short-lived association presence used for
PrivetBadge. It does not send usage events, clicks, play time, crash reports,
hardware identifiers, search history, installed-mod lists, or local logs to a
Private Client server for analytics.

## Association presence (PrivetBadge)

When an authenticated player joins a remote multiplayer server, Private Client
Core may send a heartbeat to the pinned association API
(`private-client-association.vercel.app`) so other Private Client users on that same
server can show the launcher logo next to nicknames (nametag and TAB), similar
to Lunar Client association.

Each heartbeat contains only:

- Minecraft UUID;
- Minecraft username;
- SHA-256 hash of the normalized `host:port` (never the raw hostname);
- client version and schema version.

Heartbeats repeat about every 15 seconds while connected. Presence objects
expire after about 45 seconds without refresh (filtered by upload time in
Vercel Blob). No Minecraft access token, refresh token, password,
Microsoft/Xbox credential, cookie, or authorization header is sent or stored.
Presence is not a general telemetry channel and is not used for advertising.

## Discord Status

When Discord Status is enabled, Core sends only the fixed text `Playing Private
Client`, the client version, and the local session start time to the Discord
desktop app through Discord's local IPC pipe. It never includes the Minecraft
username, UUID, server address, world name, or mod list. Discord controls whether
that activity is shared under the user's Discord privacy settings. The feature
can be disabled in Private Settings and does not contact a Private Client server.

## Data stored on this computer

Private Client stores only what is needed to run the local installation:

- launcher settings such as RAM and a manually selected Java path;
- Minecraft/Forge files downloaded from their official providers;
- installed-mod records and provider hashes;
- locally cached public mod metadata and skin images;
- a sanitized profile containing username, UUID, skin model/path, timestamp, and
  schema version;
- bounded rotating launcher/game/Core logs with secret redaction;
- staging files and backups while an atomic operation is in progress.

The default location is `%LOCALAPPDATA%\Private Client`.

The launcher does **not** store an email address, Microsoft password, 2FA code,
OAuth authorization code, access token, refresh token, Xbox token, cookie,
authorization header, or a serialized Minecraft session. In-game authentication
is an independent external component and is outside the launcher's storage.

## Network requests

Files and metadata requested by the user come directly from Mojang/Microsoft,
Forge, Modrinth, and approved skin/runtime/update/association providers listed in
`NETWORK_POLICY.md`. Those providers can observe ordinary network information,
including the source IP and requested resource.

Modrinth search history is held only in volatile UI state by default and is not
persisted.

## Logs and crashes

Logs remain local, rotate by size, and pass through patterns that redact bearer
tokens, authorization/cookie headers, passwords, client secrets, and session
identifiers. Crashes are summarized locally. Nothing is uploaded automatically.
Log export requires a deliberate user action and creates a local archive for
the user to inspect and share at their discretion.

## Delete or inspect data

Open the technical panel to view the active data and log directories. Close the
game and launcher, then delete `%LOCALAPPDATA%\Private Client` to erase all
Private Client data. Uninstalling the application does not silently delete the
game instance; this avoids destructive data loss. The uninstaller explains the
remaining local directory.

Association presence rows on the API expire automatically; leaving a server
stops heartbeats. No third-party analytics, advertising SDK, remote crash
reporter, fingerprint, affiliate program, or background data broker is included.
