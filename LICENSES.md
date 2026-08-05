# Dependency licenses

Private Client's original code uses the private source license in `LICENSE`.
Third-party components remain under their own licenses.

## Direct build/runtime dependencies

| Component | Pinned version | Source | License | Use / distribution |
|---|---:|---|---|---|
| Tauri | 2.11.x | https://github.com/tauri-apps/tauri | MIT / Apache-2.0 | Desktop runtime, compiled into launcher |
| React / React DOM | 19.2.8 | https://github.com/facebook/react | MIT | Bundled UI |
| Framer Motion | 12.43.0 | https://github.com/motiondivision/motion | MIT | Bundled UI animation |
| Zustand | 5.0.14 | https://github.com/pmndrs/zustand | MIT | Bundled local UI state |
| Zod | 4.4.3 | https://github.com/colinhacks/zod | MIT | Bundled contract validation |
| Lucide React | 1.28.0 | https://github.com/lucide-icons/lucide | ISC | Bundled icons |
| skinview3d | 3.4.2 | https://github.com/bs-community/skinview3d | MIT | Bundled 3D skin renderer |
| reqwest | 0.12.28 | https://github.com/seanmonstar/reqwest | MIT / Apache-2.0 | Compiled network adapter |
| Tokio | 1.53.1 | https://github.com/tokio-rs/tokio | MIT | Compiled async runtime |
| serde / serde_json | 1.0.229 / 1.0.151 | https://github.com/serde-rs/serde | MIT / Apache-2.0 | Compiled serialization |
| sha1 / sha2 | 0.10.6 / 0.10.9 | https://github.com/RustCrypto/hashes | MIT / Apache-2.0 | Compiled integrity checks |
| zip | 2.4.2 | https://github.com/zip-rs/zip2 | MIT | Compiled JAR validation |
| Forge | 11.15.1.2318 | https://github.com/MinecraftForge/MinecraftForge | LGPL-2.1 | Downloaded by user at runtime; not bundled |
| ForgeGradle | pinned legacy 2.1 build | https://github.com/MinecraftForge/ForgeGradle | LGPL-2.1 | Build tool only |
| JUnit | 4.13.2 | https://github.com/junit-team/junit4 | EPL-1.0 | Tests only |
| In-Game Account Switcher | 7.1.2-fo1.8.9 | https://modrinth.com/mod/in-game-account-switcher | LGPL-3.0-or-later | Exact runtime download; not bundled in installer |
| Modrinth API | v2 | https://docs.modrinth.com/api/ | Service/API | Metadata and user-selected downloads |
| Minecraft | 1.8.9 | https://www.minecraft.net/ | Microsoft/Mojang EULA | User-requested official download; never bundled |
| OptiFine | user-provided 1.8.9 file | https://optifine.net/ | OptiFine terms | Local import only; never downloaded/bundled |

## Private Pack components

Every component below is fetched at runtime from the upstream CDN, pinned to an
exact version and verified against both a SHA-1 and a SHA-512 digest. None of
them is bundled inside the installer or redistributed by this project.

| Component | Version | Source | License | Role |
| --- | --- | --- | --- | --- |
| HitDelayFix | 1.0.1 | https://github.com/ghast/HitDelayFixMod | MIT | Combat hit-delay fix |
| Animatium Legacy (OverflowAnimations) | 2.2.2 | https://modrinth.com/mod/animatium-legacy | LGPL-3.0-only | 1.7-style animations |
| Fullbright | 1.0.0 | https://modrinth.com/mod/full-bright | CC-BY-NC-ND-4.0 | Brightness control |
| FoamFix | 0.6.3a | https://modrinth.com/mod/foamfix | Custom (see project) | Memory optimization |
| No Hurt Cam | 1.0.0 | https://modrinth.com/mod/nohurtcam | MIT | Removes hurt camera tilt |
| PolyPatcher | 1.10.3 | https://modrinth.com/mod/patcher | CC-BY-NC-SA-4.0 | Bug fixes, QoL, performance |
| Phosphor Legacy Forge | 7 | https://modrinth.com/mod/phosphorlegacyforge | GPL-3.0-or-later | Lighting-engine optimization |
| Velox Caelo | 1.1.0 | https://modrinth.com/mod/veloxcaelo | All Rights Reserved | Sky/render optimization |
| CrashPatch | 2.0.2 | https://modrinth.com/mod/crashpatch | GPL-3.0 with Minecraft linking exception | Recoverable crash screen |
| Raw Input | 0.1.8 | https://modrinth.com/mod/rawinput | GPL-3.0-only | Direct mouse input |
| ServerlistBufferFixer | 1.0.1 | https://modrinth.com/mod/serverlistbufferfixer | Unlicense | Server-list ping fix |
| QuickQuit | 1.0.1 | https://modrinth.com/mod/quickquit | LGPL-3.0-only | Closable Forge loading screen |

**Commercial-distribution review required.** PolyPatcher (CC-BY-NC-SA-4.0) and
Fullbright (CC-BY-NC-ND-4.0) carry NonCommercial terms, and Velox Caelo is All
Rights Reserved. They are downloaded by the end user rather than redistributed
here, but if Private Client is distributed commercially these three need written
permission from their authors or removal from the pack. Legal sign-off on this
point is outstanding.

The build dependency graph also contains transitive packages. After dependencies
are installed, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release/generate-license-report.ps1
```

It writes machine-readable full Node and Rust reports to `reports/licenses`.
Those reports, the package lock, Cargo lock, and Java dependency report are
release evidence and must be reviewed before public distribution.

No dependency is intentionally modified or redistributed as a standalone
package. Compiled/bundled dependencies retain required copyright and license
notices.
