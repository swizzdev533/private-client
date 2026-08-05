package client.privateclient.events;

import java.time.Instant;
import java.util.Objects;

public final class CoreEvent {
    private final CoreEventType type;
    private final Instant occurredAt;
    private final String code;

    private CoreEvent(CoreEventType type, Instant occurredAt, String code) {
        this.type = Objects.requireNonNull(type, "type");
        this.occurredAt = Objects.requireNonNull(occurredAt, "occurredAt");
        this.code = validateCode(code);
    }

    public static CoreEvent of(CoreEventType type) {
        return new CoreEvent(type, Instant.now(), "");
    }

    public static CoreEvent withCode(CoreEventType type, String code) {
        return new CoreEvent(type, Instant.now(), code);
    }

    public CoreEventType getType() {
        return type;
    }

    public Instant getOccurredAt() {
        return occurredAt;
    }

    public String getCode() {
        return code;
    }

    private static String validateCode(String value) {
        String candidate = value == null ? "" : value.trim();
        if (candidate.length() > 64 || !candidate.matches("[A-Za-z0-9._-]*")) {
            throw new IllegalArgumentException("Event code must be a short, non-sensitive identifier");
        }
        return candidate;
    }
}
