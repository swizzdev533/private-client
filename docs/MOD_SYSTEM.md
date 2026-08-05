# Mod System

Managed mods have a versioned local record containing provider, project/version
IDs, exact filename, provider hashes, local SHA-512, Minecraft version, loader,
environment, license, required flag, install time, and dependency edges.

Installation plans are immutable after confirmation. Required dependencies are
resolved recursively; optional dependencies remain opt-in. Cycles, incompatible
1.8.9/Forge versions, unknown files, server-only projects, Fabric/Quilt/NeoForge,
and target filename collisions block commit.

The transaction downloads and verifies the complete graph before changing the
mods directory. Existing managed files and the database are backed up. All
renames succeed or the prior set is restored. Operations requested while the game
is running are serialized in a local queue and revalidated when the game exits.

Private Client Core and the pinned authentication integration are required. They
cannot be removed through the UI. A dependency cannot be removed while another
active managed mod references it. Unmanaged local JARs are displayed but never
silently overwritten or deleted.

OptiFine is a `local-import` record and has no download provider. The user is
responsible for obtaining it lawfully.
