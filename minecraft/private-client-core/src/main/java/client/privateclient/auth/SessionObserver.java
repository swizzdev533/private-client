package client.privateclient.auth;

import client.privateclient.events.CoreEvent;
import client.privateclient.events.CoreEventBus;
import client.privateclient.events.CoreEventType;
import client.privateclient.profile.PlayerProfile;
import client.privateclient.profile.ProfileBridge;
import client.privateclient.profile.SkinModel;
import java.io.IOException;
import java.time.Clock;
import java.time.Instant;
import java.util.Objects;
import java.util.Optional;
import java.util.UUID;

public final class SessionObserver {
    private final SessionProvider provider;
    private final SessionPolicy policy;
    private final ProfileBridge profileBridge;
    private final CoreEventBus eventBus;
    private final Clock clock;
    private final boolean profileBridgeEnabled;

    private SessionSnapshot current = SessionSnapshot.missing();
    private SessionValidation validation = new SessionValidation(SessionStatus.MISSING_SESSION);
    private boolean initialized;

    public SessionObserver(
            SessionProvider provider,
            SessionPolicy policy,
            ProfileBridge profileBridge,
            CoreEventBus eventBus,
            boolean profileBridgeEnabled) {
        this(provider, policy, profileBridge, eventBus, profileBridgeEnabled, Clock.systemUTC());
    }

    SessionObserver(
            SessionProvider provider,
            SessionPolicy policy,
            ProfileBridge profileBridge,
            CoreEventBus eventBus,
            boolean profileBridgeEnabled,
            Clock clock) {
        this.provider = Objects.requireNonNull(provider, "provider");
        this.policy = Objects.requireNonNull(policy, "policy");
        this.profileBridge = Objects.requireNonNull(profileBridge, "profileBridge");
        this.eventBus = Objects.requireNonNull(eventBus, "eventBus");
        this.profileBridgeEnabled = profileBridgeEnabled;
        this.clock = Objects.requireNonNull(clock, "clock");
    }

    public synchronized boolean refresh() throws IOException {
        SessionSnapshot captured = provider.capture();
        if (captured == null) {
            captured = SessionSnapshot.missing();
        }
        if (initialized && captured.equals(current)) {
            return false;
        }

        current = captured;
        validation = policy.evaluate(captured);
        initialized = true;
        eventBus.publish(CoreEvent.withCode(
                CoreEventType.SESSION_CHANGED,
                validation.getStatus().name().toLowerCase(java.util.Locale.ROOT)));

        if (profileBridgeEnabled) {
            if (validation.isAuthenticated()) {
                publishSafeProfile(captured);
            } else if (profileBridge.clear()) {
                eventBus.publish(CoreEvent.withCode(CoreEventType.PROFILE_CHANGED, "cleared"));
            }
        }
        return true;
    }

    public synchronized SessionSnapshot getCurrent() {
        return current;
    }

    public synchronized SessionValidation getValidation() {
        return validation;
    }

    private void publishSafeProfile(SessionSnapshot snapshot) throws IOException {
        UUID uuid = snapshot.getUuid().orElseThrow(
                () -> new IllegalStateException("Validated session has no UUID"));
        SkinModel skinModel = SkinModel.CLASSIC;
        String skinPath = "";

        Optional<PlayerProfile> existing = profileBridge.read();
        if (existing.isPresent() && uuid.equals(existing.get().getUuid())) {
            skinModel = existing.get().getSkinModel();
            skinPath = existing.get().getSkinPath();
        }

        Instant updatedAt = clock.instant();
        profileBridge.publish(new PlayerProfile(
                PlayerProfile.CURRENT_SCHEMA_VERSION,
                snapshot.getUsername(),
                uuid,
                skinModel,
                skinPath,
                updatedAt));
        eventBus.publish(CoreEvent.withCode(CoreEventType.PROFILE_CHANGED, "published"));
    }
}
