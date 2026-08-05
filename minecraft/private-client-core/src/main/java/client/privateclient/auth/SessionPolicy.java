package client.privateclient.auth;

import java.util.UUID;

public final class SessionPolicy {
    public SessionValidation evaluate(SessionSnapshot snapshot) {
        if (snapshot == null) {
            return new SessionValidation(SessionStatus.MISSING_SESSION);
        }
        if (!snapshot.getUsername().matches("[A-Za-z0-9_]{1,16}")) {
            return new SessionValidation(SessionStatus.INVALID_USERNAME);
        }
        if (!snapshot.getUuid().isPresent() || isNil(snapshot.getUuid().get())) {
            return new SessionValidation(SessionStatus.INVALID_UUID);
        }
        if (!"mojang".equals(snapshot.getSessionType())) {
            return new SessionValidation(SessionStatus.OFFLINE_SESSION_TYPE);
        }
        if (!snapshot.isCredentialPresent()) {
            return new SessionValidation(SessionStatus.MISSING_CREDENTIAL);
        }
        if (!snapshot.isProfileIdentityPresent()) {
            return new SessionValidation(SessionStatus.MISSING_PROFILE_IDENTITY);
        }
        return new SessionValidation(SessionStatus.AUTHENTICATED);
    }

    private static boolean isNil(UUID uuid) {
        return uuid.getMostSignificantBits() == 0L && uuid.getLeastSignificantBits() == 0L;
    }
}
