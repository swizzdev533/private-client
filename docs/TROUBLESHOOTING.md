# Troubleshooting

## Java not found

On PLAY the launcher first looks for a compatible 64-bit Java 8 (settings,
`JAVA_HOME`, PATH, common vendor installs). If none is found it automatically
downloads the pinned Eclipse Temurin JRE 8 into
`%LOCALAPPDATA%\Private Client\runtime\` and uses that.

If download or validation fails, check the network connection and retry PLAY, or
select a local `javaw.exe` in the technical settings panel.
Private Client rejects incompatible major versions for the 1.8.9 Forge instance.

## Forge installation failed

Open local diagnostics and the latest launcher log. Confirm network access to the
official Forge Maven host and enough free disk space. Private Client never falls
back to an unofficial mirror. Forge 1.8.9 has no headless `--installClient`
switch; Private Client validates and applies the official install profile
directly. An unknown/malformed legacy profile is rejected rather than sent to the
interactive Swing installer.

## Game does not start

Run **Repair instance**, verify Java 8, and inspect the local crash summary.
Queued mod changes are applied only after the game is stopped. A stale process
lock is removed only after the referenced PID is confirmed absent.

## Search works but install is disabled

The result may lack an exact Forge 1.8.9 client JAR, use a blocked loader, require
an incompatible dependency, be alpha-only, or need license review. Open details
for the stable reason code.

## Profile remains “Zaloguj się w grze”

Launch the game and finish the official Microsoft flow inside the pinned external
account mod. A profile appears only after Core observes an authenticated session.
The launcher has no account form and never receives tokens.

## Windows warns about the application

Local artifacts are not Authenticode-signed because this build has no publisher
certificate. Verify `SHA512SUMS.txt`. Public distribution should use signed
release artifacts.
