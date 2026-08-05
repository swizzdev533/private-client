# Contracts

The runtime wire types are defined in Rust at
`apps/launcher/src-tauri/src/contracts.rs` and mirrored/validated with Zod at
`apps/launcher/src/services/contracts.ts`.

Compatibility rules:

- persisted JSON has an integer `schemaVersion`;
- Tauri command arguments use camelCase;
- command errors expose a stable code, title-safe message, optional action, and
  local log path—never an account/session secret;
- progress events are additive and unknown fields are ignored by older UIs;
- breaking changes require a schema migration and fixture.
