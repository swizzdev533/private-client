package client.privateclient.auth;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import client.privateclient.events.CoreEventBus;
import client.privateclient.profile.ProfileBridge;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;

public final class SessionObserverTest {
    @Rule
    public final TemporaryFolder temporaryFolder = new TemporaryFolder();

    @Test
    public void publishesAuthenticatedIdentityAndClearsItWhenSessionBecomesOffline() throws Exception {
        Path dataRoot = temporaryFolder.newFolder("observer-data").toPath();
        Path profileFile = dataRoot.resolve("profiles/profile.json");
        ProfileBridge bridge = new ProfileBridge(dataRoot, profileFile);
        MutableProvider provider = new MutableProvider(new SessionSnapshot(
                "Example",
                "12345678-1234-1234-9234-1234567890ab",
                "mojang",
                true,
                true));
        CoreEventBus bus = new CoreEventBus((event, failure) -> {
        });
        SessionObserver observer = new SessionObserver(
                provider,
                new SessionPolicy(),
                bridge,
                bus,
                true);

        assertTrue(observer.refresh());
        assertEquals("Example", bridge.read().get().getUsername());
        assertTrue(Files.exists(profileFile));

        provider.snapshot = new SessionSnapshot(
                "Player",
                "12345678-1234-1234-9234-1234567890ab",
                "legacy",
                false,
                true);
        assertTrue(observer.refresh());
        assertFalse(Files.exists(profileFile));
        assertFalse(observer.getValidation().isAuthenticated());
    }

    private static final class MutableProvider implements SessionProvider {
        private SessionSnapshot snapshot;

        private MutableProvider(SessionSnapshot snapshot) {
            this.snapshot = snapshot;
        }

        @Override
        public SessionSnapshot capture() {
            return snapshot;
        }
    }
}
