package client.privateclient.auth;

import java.util.Objects;

public final class SessionValidation {
    private final SessionStatus status;

    public SessionValidation(SessionStatus status) {
        this.status = Objects.requireNonNull(status, "status");
    }

    public SessionStatus getStatus() {
        return status;
    }

    public boolean isAuthenticated() {
        return status == SessionStatus.AUTHENTICATED;
    }

    public String getUserMessage() {
        if (isAuthenticated()) {
            return "Session ready.";
        }
        return "Sign in inside the game before joining multiplayer.";
    }
}
