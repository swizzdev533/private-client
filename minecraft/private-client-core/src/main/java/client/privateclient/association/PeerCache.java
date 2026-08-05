package client.privateclient.association;

import java.util.Collections;
import java.util.HashSet;
import java.util.Locale;
import java.util.Set;
import java.util.UUID;

/**
 * Thread-safe peer cache with soft retention. Indexes by Mojang/session UUID and
 * by lowercase username so badges still resolve on offline-mode servers where
 * entity UUIDs differ from account UUIDs.
 */
public final class PeerCache {
    public static final long DEFAULT_SOFT_RETENTION_MS = 60_000L;

    private final long softRetentionMs;
    private final Object lock = new Object();
    private Set<UUID> peers = new HashSet<UUID>();
    private Set<String> usernames = new HashSet<String>();
    private long lastSuccessAtMs = 0L;

    public PeerCache() {
        this(DEFAULT_SOFT_RETENTION_MS);
    }

    public PeerCache(long softRetentionMs) {
        if (softRetentionMs < 0L) {
            throw new IllegalArgumentException("softRetentionMs must be >= 0");
        }
        this.softRetentionMs = softRetentionMs;
    }

    public void replaceAll(Set<UUID> next, long nowMs) {
        replaceAll(next, Collections.<String>emptySet(), nowMs);
    }

    public void replaceAll(Set<UUID> nextUuids, Set<String> nextUsernames, long nowMs) {
        if (nextUuids == null || nextUsernames == null) {
            throw new IllegalArgumentException("peer sets are required");
        }
        synchronized (lock) {
            peers = new HashSet<UUID>(nextUuids);
            usernames = normalizeNames(nextUsernames);
            lastSuccessAtMs = nowMs;
        }
    }

    public void merge(UUID uuid, long nowMs) {
        merge(uuid, null, nowMs);
    }

    public void merge(UUID uuid, String username, long nowMs) {
        synchronized (lock) {
            if (uuid != null) {
                peers.add(uuid);
            }
            String normalized = normalizeName(username);
            if (normalized != null) {
                usernames.add(normalized);
            }
            if (lastSuccessAtMs == 0L) {
                lastSuccessAtMs = nowMs;
            }
        }
    }

    public void clear() {
        synchronized (lock) {
            peers = new HashSet<UUID>();
            usernames = new HashSet<String>();
            lastSuccessAtMs = 0L;
        }
    }

    public boolean isAssociated(UUID uuid, long nowMs) {
        return isAssociated(uuid, null, nowMs);
    }

    public boolean isAssociated(UUID uuid, String username, long nowMs) {
        synchronized (lock) {
            if (peers.isEmpty() && usernames.isEmpty()) {
                return false;
            }
            if (lastSuccessAtMs > 0L && nowMs - lastSuccessAtMs > softRetentionMs) {
                return false;
            }
            if (uuid != null && peers.contains(uuid)) {
                return true;
            }
            String normalized = normalizeName(username);
            return normalized != null && usernames.contains(normalized);
        }
    }

    public Set<UUID> snapshot(long nowMs) {
        synchronized (lock) {
            if (lastSuccessAtMs > 0L && nowMs - lastSuccessAtMs > softRetentionMs) {
                return Collections.emptySet();
            }
            return Collections.unmodifiableSet(new HashSet<UUID>(peers));
        }
    }

    public static UUID parseUuid(String value) {
        if (value == null || value.trim().isEmpty()) {
            return null;
        }
        try {
            return UUID.fromString(value.trim().toLowerCase(Locale.ROOT));
        } catch (IllegalArgumentException ignored) {
            return null;
        }
    }

    private static Set<String> normalizeNames(Set<String> values) {
        Set<String> normalized = new HashSet<String>();
        for (String value : values) {
            String name = normalizeName(value);
            if (name != null) {
                normalized.add(name);
            }
        }
        return normalized;
    }

    private static String normalizeName(String username) {
        if (username == null) {
            return null;
        }
        String trimmed = username.trim();
        if (trimmed.isEmpty() || trimmed.length() > 16) {
            return null;
        }
        if (!trimmed.matches("^[A-Za-z0-9_]{1,16}$")) {
            return null;
        }
        return trimmed.toLowerCase(Locale.ROOT);
    }
}
