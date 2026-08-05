# Development

Run `pnpm install`, then `pnpm dev`. Tauri starts Vite and the Rust host. The UI
also has a browser-safe preview mode for component tests, but release behavior is
always driven by Tauri commands.

Keep domain logic outside React components and Tauri command functions. UI
contracts live in the frontend service layer; Rust wire types live in
`src-tauri/src/contracts.rs`. Persisted structures always have `schemaVersion`.

Before handing off a change:

```powershell
pnpm lint
pnpm typecheck
pnpm test
cargo fmt --manifest-path apps/launcher/src-tauri/Cargo.toml --check
cargo clippy --manifest-path apps/launcher/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path apps/launcher/src-tauri/Cargo.toml
```

Use fixtures or a local server in tests; unit/integration tests must not depend on
the production Modrinth, Mojang, Forge, or Microsoft services.
