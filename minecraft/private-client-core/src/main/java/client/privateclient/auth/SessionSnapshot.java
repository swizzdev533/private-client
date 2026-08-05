package client.privateclient.auth;

import java.util.Locale;
import java.util.Objects;
import java.util.Optional;
import java.util.UUID;

public final class SessionSnapshot {
    private final String username;
    private final UUID uuid;
    private final String sessionType;
    private final boolean credentialPresent;
    private final boolean profileIdentityPresent;

    public SessionSnapshot(
            String username,
            String uuid,
            String sessionType,
            boolean credentialPresent,
            boolean profileIdentityPresent) {
        this.username = username == null ? "" : username.trim();
        this.uuid = parseUuid(uuid).orElse(null);
        this.sessionType = sessionType == null
                ? ""
                : sessionType.trim().toLowerCase(Locale.ROOT);
        this.credentialPresent = credentialPresent;
        this.profileIdentityPresent = profileIdentityPresent;
    }

    public static SessionSnapshot missing() {
        return new SessionSnapshot("", "", "", false, false);
    }

    public String getUsername() {
        return username;
    }

    public Optional<UUID> getUuid() {
        return Optional.ofNullable(uuid);
    }

    public String getSessionType() {
        return sessionType;
    }

    public boolean isCredentialPresent() {
        return credentialPresent;
    }

    public boolean isProfileIdentityPresent() {
        return profileIdentityPresent;
    }

    @Override
    public boolean equals(Object other) {
        if (this == other) {
            return true;
        }
        if (!(other instanceof SessionSnapshot)) {
            return false;
        }
        SessionSnapshot that = (SessionSnapshot) other;
        return credentialPresent == that.credentialPresent
                && profileIdentityPresent == that.profileIdentityPresent
                && username.equals(that.username)
                && Objects.equals(uuid, that.uuid)
                && sessionType.equals(that.sessionType);
    }

    @Override
    public int hashCode() {
        return Objects.hash(
                username,
                uuid,
                sessionType,
                credentialPresent,
                profileIdentityPresent);
    }

    @Override
    public String toString() {
        return "SessionSnapshot{"
                + "usernamePresent=" + !username.isEmpty()
                + ", uuidPresent=" + (uuid != null)
                + ", sessionType='" + sessionType + '\''
                + ", credentialPresent=" + credentialPresent
                + ", profileIdentityPresent=" + profileIdentityPresent
                + '}';
    }

    private static Optional<UUID> parseUuid(String value) {
        if (value == null) {
            return Optional.empty();
        }
        String candidate = value.trim();
        if (candidate.matches("[0-9a-fA-F]{32}")) {
            candidate = candidate.substring(0, 8)
                    + "-" + candidate.substring(8, 12)
                    + "-" + candidate.substring(12, 16)
                    + "-" + candidate.substring(16, 20)
                    + "-" + candidate.substring(20);
        }
        try {
            return Optional.of(UUID.fromString(candidate));
        } catch (IllegalArgumentException exception) {
            return Optional.empty();
        }
    }
}
