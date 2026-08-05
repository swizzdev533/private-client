# Discord Rich Presence

Private Client publishes a minimal activity to the local Discord desktop client:

- `Playing Private Client`
- `Minecraft 1.8.9`
- session elapsed time
- the `private_client` application asset

It never publishes a username, UUID, world, server address, or mod list.

## Release setup

1. Create a Discord application named `Private Client` in the Discord Developer
   Portal.
2. In Rich Presence assets, upload the Private Client logo with the key
   `private_client`.
3. The official public Application ID `1533770052908744755` is embedded by
   default. `PRIVATE_CLIENT_DISCORD_APPLICATION_ID` may override it for a
   development build before running `pnpm build`.
4. Invite release testers or complete Discord's approval process when required.

The ID is embedded into the Core JAR as public configuration; it is not a token
or secret. A missing or malformed override disables the integration safely.
Developers can override it at runtime with the same environment variable or the
Java system property `privateclient.discord.applicationId`.

Users can turn the feature on or off immediately from **Private Settings**. The
implementation communicates only through `discord-ipc-*` on Windows and stores
no Discord account material.
