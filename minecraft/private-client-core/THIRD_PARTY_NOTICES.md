# Third-party notices

Private Client Core compiles against APIs supplied by a legal Minecraft 1.8.9
installation and Minecraft Forge `11.15.1.2318`. Neither binary is committed to
this directory or embedded in the output JAR.

The Gradle wrapper bootstrap is included so the exact Gradle 2.7 distribution
can be fetched from the official Gradle service and verified by SHA-256.
ForgeGradle, Forge userdev and MCP mappings are fetched only into the local
build cache. Their fixed coordinates and verification hashes are documented in
the README and build script.

The output JAR contains Private Client Core classes, `mcmod.info`, localization
text and its manifest. It does not contain third-party classes or assets.
