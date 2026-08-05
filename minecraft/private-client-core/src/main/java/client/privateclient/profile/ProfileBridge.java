package client.privateclient.profile;

import client.privateclient.security.AtomicFiles;
import client.privateclient.security.SafePaths;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.time.Clock;
import java.util.Optional;

public final class ProfileBridge {
    private static final long MAX_PROFILE_BYTES = 256L * 1024L;

    private final Path dataRoot;
    private final Path profileFile;
    private final ProfileCodec codec;
    private final Clock clock;

    public ProfileBridge(Path dataRoot, Path profileFile) {
        this(dataRoot, profileFile, new ProfileCodec(), Clock.systemUTC());
    }

    ProfileBridge(Path dataRoot, Path profileFile, ProfileCodec codec, Clock clock) {
        if (dataRoot == null || profileFile == null || codec == null || clock == null) {
            throw new IllegalArgumentException("Bridge paths, codec and clock are required");
        }
        this.dataRoot = SafePaths.normalizeRoot(dataRoot);
        this.profileFile = profileFile.toAbsolutePath().normalize();
        this.codec = codec;
        this.clock = clock;
        if (!this.profileFile.startsWith(this.dataRoot)) {
            throw new IllegalArgumentException("Profile file must stay inside the data root");
        }
    }

    public synchronized void publish(PlayerProfile profile) throws IOException {
        if (profile == null) {
            throw new IllegalArgumentException("Profile is required");
        }
        if (!profile.getSkinPath().isEmpty()) {
            SafePaths.resolveRelative(dataRoot, profile.getSkinPath());
        }
        AtomicFiles.writeUtf8(profileFile, codec.encode(profile));
    }

    public synchronized Optional<PlayerProfile> read() throws IOException {
        if (!Files.exists(profileFile, LinkOption.NOFOLLOW_LINKS)) {
            return Optional.empty();
        }
        if (Files.isSymbolicLink(profileFile)) {
            throw new IOException("Profile file must not be a symbolic link");
        }
        if (Files.size(profileFile) > MAX_PROFILE_BYTES) {
            AtomicFiles.quarantine(profileFile, clock);
            return Optional.empty();
        }
        String json = new String(Files.readAllBytes(profileFile), StandardCharsets.UTF_8);
        try {
            return Optional.of(codec.decode(json));
        } catch (RuntimeException invalidProfile) {
            AtomicFiles.quarantine(profileFile, clock);
            return Optional.empty();
        }
    }

    public synchronized boolean clear() throws IOException {
        if (Files.isSymbolicLink(profileFile)) {
            throw new IOException("Profile file must not be a symbolic link");
        }
        return Files.deleteIfExists(profileFile);
    }

    public Path getProfileFile() {
        return profileFile;
    }
}
