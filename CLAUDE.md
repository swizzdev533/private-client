@AGENTS.md

# Claude Code workflow

- Treat `AGENTS.md` as the authoritative project instructions.
- Use `pnpm` and preserve the pinned lockfile and build dependencies.
- Preserve unrelated changes. Do not commit, push, publish, or deploy unless the user explicitly asks.
- Use one Orca worktree per task once this folder is a Git repository.
- Run checks appropriate to the changed area and always finish completed work with `pnpm build`.
- For high-risk trust-boundary changes, include the architecture and security justification plus tests required by `AGENTS.md`.
