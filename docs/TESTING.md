# Testing

The verification stack contains Vitest/Testing Library for UI and stores, Rust
unit/integration tests for security and domains, Java/JUnit tests for pure Core
logic, and Playwright smoke flows for the packaged UI contract.

Network tests use fixtures or a loopback HTTP server. They never require a live
production provider. Important cases include traversal/reparse points, redirect
allowlists, size/time limits, hash mismatch, malformed JARs, dependency
cycles/conflicts, atomic rollback, queueing, database/profile migration, Java
version/architecture, RAM bounds, process locks, crash classification, reduced
motion, and OptiFine validation.

Manual release gates:

- install/uninstall on a clean Windows x64 VM;
- first-run WebView2 behavior;
- Java absent, Java 8, and incompatible Java scenarios;
- legal Minecraft 1.8.9 download/repair;
- Forge installer execution;
- licensed Microsoft in-game authentication;
- profile/skin refresh;
- Modrinth install/update/remove against at least two reviewed compatible mods;
- offline and interrupted-operation recovery;
- code signature and SmartScreen reputation.

Tests cannot substitute for a publisher certificate or licensed account.
