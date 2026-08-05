package client.privateclient.auth;

import java.util.Objects;

public final class ConnectionDecision {
    private final boolean allowed;
    private final SessionStatus sessionStatus;

    private ConnectionDecision(boolean allowed, SessionStatus sessionStatus) {
        this.allowed = allowed;
        this.sessionStatus = Objects.requireNonNull(sessionStatus, "sessionStatus");
    }

    public static ConnectionDecision allowed(SessionStatus status) {
        return new ConnectionDecision(true, status);
    }

    public static ConnectionDecision blocked(SessionStatus status) {
        return new ConnectionDecision(false, status);
    }

    public boolean isAllowed() {
        return allowed;
    }

    public SessionStatus getSessionStatus() {
        return sessionStatus;
    }
}
