package client.privateclient.profile;

import client.privateclient.security.SafePaths;
import java.time.Instant;
import java.util.Objects;
import java.util.UUID;

public final class PlayerProfile {
    public static final int CURRENT_SCHEMA_VERSION = 1;

    private final int schemaVersion;
    private final String username;
    private final UUID uuid;
    private final SkinModel skinModel;
    private final String skinPath;
    private final Instant updatedAt;

    public PlayerProfile(
            int schemaVersion,
            String username,
            UUID uuid,
            SkinModel skinModel,
            String skinPath,
            Instant updatedAt) {
        if (schemaVersion != CURRENT_SCHEMA_VERSION) {
            throw new IllegalArgumentException("Unsupported profile schema: " + schemaVersion);
        }
        String normalizedUsername = username == null ? "" : username.trim();
        if (!normalizedUsername.matches("[A-Za-z0-9_]{1,16}")) {
            throw new IllegalArgumentException("Invalid Minecraft username");
        }
        UUID validatedUuid = Objects.requireNonNull(uuid, "uuid");
        if (validatedUuid.getMostSignificantBits() == 0L && validatedUuid.getLeastSignificantBits() == 0L) {
            throw new IllegalArgumentException("Nil UUID is not a player identity");
        }

        String normalizedSkinPath = SafePaths.normalizeRelative(skinPath == null ? "" : skinPath);
        if (!normalizedSkinPath.isEmpty()) {
            String expected = "cache/profiles/" + validatedUuid.toString() + "/skin.png";
            if (!expected.equals(normalizedSkinPath)) {
                throw new IllegalArgumentException("Skin path must point to the profile's local skin cache");
            }
        }

        this.schemaVersion = schemaVersion;
        this.username = normalizedUsername;
        this.uuid = validatedUuid;
        this.skinModel = Objects.requireNonNull(skinModel, "skinModel");
        this.skinPath = normalizedSkinPath;
        this.updatedAt = Objects.requireNonNull(updatedAt, "updatedAt");
    }

    public int getSchemaVersion() {
        return schemaVersion;
    }

    public String getUsername() {
        return username;
    }

    public UUID getUuid() {
        return uuid;
    }

    public SkinModel getSkinModel() {
        return skinModel;
    }

    public String getSkinPath() {
        return skinPath;
    }

    public Instant getUpdatedAt() {
        return updatedAt;
    }
}
