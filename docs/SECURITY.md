# Security

## Threat model

The primary untrusted inputs are provider JSON, URLs and redirects, downloaded
JAR/ZIP content, local import paths/files, profile files written by another
process, and frontend command input. An attacker may control a Modrinth project,
tamper with local files, race an install, supply a path traversal name, or place
a reparse point inside a writable directory. Account secrets are explicitly
outside the launcher trust boundary.

## Filesystem controls

- All writable paths derive from one per-user root.
- Project/version identifiers use bounded ASCII formats.
- Provider filenames are reduced to one validated leaf; reserved Windows names,
  alternate separators, control characters, trailing dots/spaces, and absolute
  paths are rejected.
- Canonical paths must remain under their intended root.
- Reparse points/symlinks are rejected at trust boundaries.
- ZIP entries are inspected without extracting arbitrary paths; absolute paths,
  `..`, excessive counts/sizes, encrypted entries, and suspicious ratios fail.
- Staging and targets share a volume so the final rename is atomic.
- Local databases/profile/config use temporary-write + replace and retain a
  bounded backup during migration.

## Downloads

Only HTTPS allowlisted hosts are accepted. Each redirect is revalidated. Clients
have connect/read/overall timeouts, redirect and response-size limits, and stream
to a random staging file. Expected provider hashes are checked before commit and
a local SHA-512 is recorded. A Modrinth download URL must use the official CDN.

JAR validation confirms a ZIP/JAR container, bounded central directory, manifest
presence, at least one class/resource signal, and no traversal names. This is a
structural safety check, not a claim that arbitrary mod bytecode is benign.
Dynamic results are therefore labeled `FROM MODRINTH`.

## Process controls

The frontend cannot supply an executable or raw argument vector. Backend code can
only launch closed operations: detect a Java executable, run the exact verified
Forge installer, start the prepared Minecraft version, or open an approved local
directory. Arguments are constructed from validated manifests and local paths.
Downloaded mod JARs are never executed with `java -jar`.

## Tauri boundary

The app exposes a small invoke allowlist. There is no shell plugin, generic
filesystem plugin, arbitrary HTTP plugin, global-open URL command, or broad CSP
exception. The content security policy allows only bundled UI resources. Native
dialogs return paths to a narrow Rust import command that revalidates them.

## Secret redaction

Logs replace values adjacent to `authorization`, `bearer`, `access_token`,
`refresh_token`, `password`, `cookie`, `client_secret`, and `session_id`.
Query strings are stripped from URLs before logging. Full session objects,
request headers, OAuth responses, and environment dumps are never logged.

## Updates and releases

A release update is accepted only when its bounded manifest validates, version
is newer, platform matches, artifact HTTPS host is allowed, size is sane, and
SHA-512/signature matches. Updates install while the game is stopped, back up
the previous version, then use atomic replacement and post-launch rollback.
The default distribution has updates disabled until the owner supplies a signing
key and release host.

Build outputs in this workspace are unsigned because no publisher certificate is
available. Hashes establish integrity after local build but not publisher
identity; public releases must be Authenticode-signed and tested with SmartScreen.

## Vulnerability reports

Do not include account data, tokens, or third-party copyrighted files in a
report. Provide a minimal reproduction and the affected version through the
private maintainer channel chosen by the repository owner. This project does not
operate a public collection endpoint.
