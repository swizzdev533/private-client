# Release

1. Update pinned versions, manifests, notices, changelog, and architecture notes.
2. Run the full local build and verification. Confirm that the curated launcher
   comes from `target\x86_64-pc-windows-msvc\release`; never package
   `target\release` from a GNU build.
3. Run dependency/license and secret scans in CI.
4. Test install, first run, game preparation, real licensed authentication,
   profile, mod operations, crash recovery, update rollback, and uninstall in a
   clean Windows VM.
5. Authenticode-sign `PrivateClient.exe` and `setup.exe`.
6. Regenerate the updater signature over the Authenticode-signed installer, then
   run `pnpm release:manifest -Tag v<version>`. Signing after the `.sig` is
   produced invalidates it and the updater will reject the artifact.
7. Recalculate SHA-512 after signing and archive the SBOM/test reports.
8. Publish the GitHub release with the installer, its `.sig`, and `latest.json`
   as assets. See `UPDATER.md` for the full procedure and trust boundary.
9. Require a conscious maintainer approval before publishing.

The installer never contains Minecraft, Forge, OptiFine, account data, or
third-party mod JARs. Do not publish an unsigned artifact as a trusted public
release. `pnpm verify:release` must report an x64 PE32+ launcher with no
`WebView2Loader.dll` import before signing.
