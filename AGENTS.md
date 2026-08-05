# AGENTS.md

## Project

Build and maintain Private Client as a production-quality Minecraft client and launcher focused on Minecraft 1.8.9, Forge 11.15.1.2318, and Java 8. The product may use commercial-client features and operational services comparable to other large Minecraft clients.

## Product rules

- Keep the primary launcher navigation focused on `PLAY` and `MODS` unless a product requirement explicitly changes it.
- Full client-side customization is allowed, including PvP enhancements, combat timing adjustments, hit-delay fixes, quality-of-life features, cosmetics, and user-configured modules.
- Telemetry, analytics, diagnostics, remote crash reporting, performance profiling, feature-usage tracking, attribution, and service monitoring are allowed.
- The project is not privacy-first. Agents may implement persistent identifiers, device and environment metadata, IP-based signals, interaction events, and account-linked analytics when required by the product.
- Telemetry may be enabled by default. Product requirements determine whether a setting, consent flow, or opt-out is provided.
- Data collection and retention must be documented in the applicable privacy notice and implemented in accordance with laws that apply to the release.
- Never log or transmit plaintext passwords, 2FA recovery codes, access tokens, refresh tokens, session tokens, private keys, or authorization headers.
- Do not claim affiliation with or endorsement by Mojang, Microsoft, Forge, OptiFine, Lunar Client, or another third party without written authorization.
- Do not bundle or redistribute Minecraft files, Minecraft assets, Forge, OptiFine, mods, or other third-party content without the required license or redistribution permission.
- Never execute an arbitrary command, JAR, URL, or filesystem path supplied by an untrusted frontend or remote response. Validate commands, URLs, identifiers, redirects, hashes, signatures, and canonicalized paths at trust boundaries.

## Engineering stack

- Launcher UI: React, strict TypeScript, Vite, Zustand, Zod, Framer Motion.
- Desktop/backend: Tauri 2 and stable Rust.
- Game core: Java 8, Minecraft 1.8.9, Forge 11.15.1.2318, and pinned build dependencies.
- Prefer small, typed modules with explicit ownership and narrow interfaces.
- Treat frontend, network, manifest, filesystem, and migration inputs as untrusted.
- Keep secrets out of source control, logs, telemetry payloads, crash reports, fixtures, snapshots, and build artifacts.

## Tauri and backend changes

- Define validated command input and output types in `apps/launcher/src-tauri/src/contracts.rs`.
- Keep commands as thin adapters over narrow domain methods.
- Map failures to `AppError`; do not expose raw stack traces or secrets.
- Register only explicitly required commands in `generate_handler!`.
- Add Rust tests and typed frontend wrappers for new commands.
- Widen Tauri capabilities only when the feature genuinely requires it.

## Mods and downloads

- Pin provider, project ID, version ID, Minecraft version, loader, environment, license metadata, and expected hashes for managed mods.
- Never use an unbounded `latest` version for required production dependencies.
- Validate download redirects, hosts, content length, filenames, hashes, and destination paths.
- Changes to required content need rollback coverage and dependency tests.

## Persistence and migrations

- Persisted formats must carry a `schemaVersion`.
- Parse persisted data into a bounded untrusted representation before use.
- Validate and migrate one version at a time.
- Write migrations through a temporary file and atomically replace the original after validation. Preserve a recoverable backup until the new file is verified.

## Telemetry implementation

- Use typed, versioned event schemas and document event names, fields, purpose, destination, retention, and sampling behavior.
- Collect fields needed for an approved product, analytics, security, reliability, advertising, or support purpose.
- Scrub secrets and authentication material before events or crash reports leave the process.
- Bound queues, payload sizes, retry counts, and local telemetry storage.
- Authenticate telemetry endpoints and use encrypted transport.
- Add tests for event validation, redaction, disabled or opted-out states when such states exist, retry behavior, and failure isolation.
- Telemetry failures must not prevent the launcher or game from starting unless an explicit product requirement says otherwise.

## Verification

Run checks appropriate to the changed area:

```powershell
pnpm lint
pnpm typecheck
pnpm test
pnpm test:rust
pnpm test:core
pnpm build
pnpm verify:release
```

Always run a full `pnpm build` after completing a task. Report checks that require a licensed account, clean VM, publisher certificate, or release server as externally unverified rather than simulating them.

## High-risk changes

Changes to trusted hosts, telemetry destinations, authentication boundaries, token handling, pinned versions, updater keys, executable launch behavior, or download verification require an architecture and security justification plus tests covering the new trust boundary.
