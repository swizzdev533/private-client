# Changelog

## Unreleased

- Added signed launcher self-update over a pinned GitHub Releases channel, with
  downgrade/replay rejection, bounded release notes, and refusal to update while
  Minecraft is running.
- Enabled the automatic update-check setting; it gates the startup check only
  and installation still requires an explicit user action.
- Added `pnpm release:manifest` to build and validate the `latest.json` manifest
  from a real signed installer.
- Fixed the in-game account switcher reopening every time the player returned to
  the main menu; it now appears once per game session.

## 1.0.0 - 2026-07-30

- Initial Windows release of the Private Client launcher.
- Added isolated Minecraft 1.8.9 and Forge 11.15.1.2318 preparation.
- Added local Java 8 detection, launch state reporting, logs and diagnostics.
- Added Modrinth Forge 1.8.9 search and transactional mod management.
- Added manual OptiFine import without redistribution.
- Added file-based, token-free player profile bridge and Private Client Core.
- Added monochrome PLAY/MODS interface, reduced-motion support and local-only settings.
- Added NSIS packaging and SHA-512 release verification.
