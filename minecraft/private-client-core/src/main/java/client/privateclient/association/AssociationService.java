package client.privateclient.association;

import client.privateclient.association.net.AssociationHttpClient;
import client.privateclient.association.net.PresencePayload;
import client.privateclient.auth.SessionObserver;
import client.privateclient.auth.SessionSnapshot;
import client.privateclient.logging.SafeLogger;
import java.util.Collections;
import java.util.HashSet;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * Server-scoped association presence: heartbeat + peer cache. Failures never
 * propagate to the game thread.
 *
 * <p>On join (and when the in-world identity first appears) presence uses a short
 * burst poll so peer badges appear within ~1s, then settles to a slower cadence.
 */
public final class AssociationService {
    /** Steady-state presence refresh while staying on a server. */
    public static final long HEARTBEAT_INTERVAL_MS = 15_000L;
    /** Fast poll right after join so peers see each other like Lunar. */
    public static final long BURST_INTERVAL_MS = 500L;
    public static final long BURST_DURATION_MS = 8_000L;

    private final SessionObserver sessionObserver;
    private final AssociationHttpClient httpClient;
    private final PeerCache peerCache;
    private final SafeLogger log;
    private final ScheduledExecutorService scheduler;
    private final AtomicBoolean running = new AtomicBoolean(false);
    private final Object hashLock = new Object();
    private final Object scheduleLock = new Object();

    private Set<String> activeServerHashes = new HashSet<String>();
    private volatile ScheduledFuture<?> heartbeatFuture;
    private volatile long burstUntilMs;
    /**
     * In-world identity other clients see (entity GameProfile). On offline-mode
     * servers this differs from the launcher/session UUID — presence must publish
     * the entity UUID or peers never match.
     */
    private volatile UUID publishedUuid;
    private volatile String publishedUsername;

    public AssociationService(SessionObserver sessionObserver, SafeLogger log) {
        this(sessionObserver, new AssociationHttpClient(), new PeerCache(), log);
    }

    AssociationService(
            SessionObserver sessionObserver,
            AssociationHttpClient httpClient,
            PeerCache peerCache,
            SafeLogger log) {
        this.sessionObserver = sessionObserver;
        this.httpClient = httpClient;
        this.peerCache = peerCache;
        this.log = log;
        this.scheduler = Executors.newSingleThreadScheduledExecutor(new ThreadFactory() {
            @Override
            public Thread newThread(Runnable runnable) {
                Thread thread = new Thread(runnable, "private-client-association");
                thread.setDaemon(true);
                return thread;
            }
        });
    }

    public void onMultiplayerJoined(String host, int port) {
        registerServerEndpoint(host, port);
    }

    /**
     * Registers an additional server identity (list address and/or resolved
     * remote IP:port). Presence is published under every registered hash so
     * peers still match when one client used an SRV name and another used an IP.
     */
    public void registerServerEndpoint(String host, int port) {
        try {
            String hash = ServerHash.of(host, port);
            boolean first;
            synchronized (hashLock) {
                first = activeServerHashes.isEmpty();
                activeServerHashes.add(hash);
            }
            if (first) {
                running.set(true);
                startBurstHeartbeat();
            } else if (running.get()) {
                // New alias (e.g. resolved IP) — pulse immediately under all hashes.
                requestImmediatePulse();
            }
        } catch (RuntimeException exception) {
            log.warn("Association join ignored: invalid server identity");
        }
    }

    public void onDisconnected() {
        running.set(false);
        burstUntilMs = 0L;
        synchronized (hashLock) {
            activeServerHashes = new HashSet<String>();
        }
        publishedUuid = null;
        publishedUsername = null;
        synchronized (scheduleLock) {
            ScheduledFuture<?> future = heartbeatFuture;
            if (future != null) {
                future.cancel(false);
                heartbeatFuture = null;
            }
        }
        peerCache.clear();
    }

    /**
     * Called from the client thread with the local player's in-world GameProfile.
     */
    public void updatePublishedIdentity(UUID uuid, String username) {
        if (uuid == null) {
            return;
        }
        UUID previous = publishedUuid;
        publishedUuid = uuid;
        if (username != null && !username.trim().isEmpty()) {
            publishedUsername = username.trim();
        }
        if (!running.get()) {
            return;
        }
        peerCache.merge(uuid, username, System.currentTimeMillis());
        // First in-world identity (or UUID change) — announce immediately.
        if (previous == null || !previous.equals(uuid)) {
            startBurstHeartbeat();
        }
    }

    public void mergeLocalPeer(UUID uuid) {
        if (uuid == null || !running.get()) {
            return;
        }
        peerCache.merge(uuid, System.currentTimeMillis());
    }

    public boolean shouldShowBadge(UUID uuid) {
        return shouldShowBadge(uuid, null);
    }

    public boolean shouldShowBadge(UUID uuid, String username) {
        if (uuid == null && (username == null || username.trim().isEmpty())) {
            return false;
        }
        SessionSnapshot self = sessionObserver.getCurrent();
        // Local Private Client player always gets the badge (Lunar-style self logo).
        if (uuid != null && self.getUuid().isPresent() && uuid.equals(self.getUuid().get())) {
            return true;
        }
        UUID localPublished = publishedUuid;
        if (uuid != null && localPublished != null && uuid.equals(localPublished)) {
            return true;
        }
        if (username != null
                && !username.isEmpty()
                && username.equalsIgnoreCase(self.getUsername())) {
            return true;
        }
        String localName = publishedUsername;
        if (username != null
                && localName != null
                && username.equalsIgnoreCase(localName)) {
            return true;
        }
        if (!sessionObserver.getValidation().isAuthenticated()) {
            return false;
        }
        return peerCache.isAssociated(uuid, username, System.currentTimeMillis());
    }

    public PeerCache getPeerCache() {
        return peerCache;
    }

    public void shutdown() {
        onDisconnected();
        scheduler.shutdownNow();
    }

    private void startBurstHeartbeat() {
        burstUntilMs = System.currentTimeMillis() + BURST_DURATION_MS;
        synchronized (scheduleLock) {
            ScheduledFuture<?> previous = heartbeatFuture;
            if (previous != null) {
                previous.cancel(false);
            }
            heartbeatFuture = scheduler.schedule(new HeartbeatTick(), 0L, TimeUnit.MILLISECONDS);
        }
    }

    private void requestImmediatePulse() {
        burstUntilMs = Math.max(burstUntilMs, System.currentTimeMillis() + BURST_DURATION_MS);
        scheduler.execute(new Runnable() {
            @Override
            public void run() {
                pulse();
            }
        });
    }

    private final class HeartbeatTick implements Runnable {
        @Override
        public void run() {
            if (!running.get()) {
                return;
            }
            pulse();
            if (!running.get()) {
                return;
            }
            long delay = System.currentTimeMillis() < burstUntilMs
                    ? BURST_INTERVAL_MS
                    : HEARTBEAT_INTERVAL_MS;
            synchronized (scheduleLock) {
                if (!running.get()) {
                    return;
                }
                heartbeatFuture = scheduler.schedule(this, delay, TimeUnit.MILLISECONDS);
            }
        }
    }

    private void pulse() {
        if (!running.get()) {
            return;
        }
        try {
            // Refresh session so heartbeat uses a live authenticated snapshot.
            sessionObserver.refresh();
        } catch (Exception ignored) {
            // Soft-fail; pulse will retry on the next interval.
        }
        if (!sessionObserver.getValidation().isAuthenticated()) {
            return;
        }
        Set<String> hashes;
        synchronized (hashLock) {
            if (activeServerHashes.isEmpty()) {
                return;
            }
            hashes = new HashSet<String>(activeServerHashes);
        }
        SessionSnapshot session = sessionObserver.getCurrent();
        if (!session.getUuid().isPresent()) {
            return;
        }
        // Prefer the in-world UUID other clients render against.
        UUID presenceUuid = publishedUuid != null ? publishedUuid : session.getUuid().get();
        String username = publishedUsername != null ? publishedUsername : session.getUsername();
        if (username == null || username.isEmpty()) {
            return;
        }
        Set<UUID> nextUuids = new HashSet<UUID>();
        Set<String> nextNames = new HashSet<String>();
        nextUuids.add(presenceUuid);
        nextUuids.add(session.getUuid().get());
        nextNames.add(username);
        if (session.getUsername() != null && !session.getUsername().isEmpty()) {
            nextNames.add(session.getUsername());
        }

        int successCount = 0;
        for (String serverHash : hashes) {
            try {
                String body = PresencePayload.buildRequestJson(
                        presenceUuid,
                        username,
                        serverHash,
                        AssociationEndpoints.CLIENT_VERSION);
                Map<UUID, String> peers = httpClient.heartbeatPeers(body);
                nextUuids.addAll(peers.keySet());
                for (String peerName : peers.values()) {
                    if (peerName != null && !peerName.isEmpty()) {
                        nextNames.add(peerName);
                    }
                }
                successCount++;
            } catch (Exception exception) {
                log.warn("Association heartbeat failed (soft): "
                        + exception.getClass().getSimpleName().toLowerCase(Locale.ROOT));
            }
        }

        if (successCount > 0) {
            peerCache.replaceAll(nextUuids, nextNames, System.currentTimeMillis());
        }
    }

    /** Test seam */
    void pulseForTest() {
        pulse();
    }

    ExecutorService executorForTest() {
        return scheduler;
    }

    Set<String> hashesForTest() {
        synchronized (hashLock) {
            return Collections.unmodifiableSet(new HashSet<String>(activeServerHashes));
        }
    }
}
