package client.privateclient.association;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import client.privateclient.association.net.AssociationHttpClient;
import client.privateclient.auth.SessionObserver;
import client.privateclient.auth.SessionPolicy;
import client.privateclient.auth.SessionProvider;
import client.privateclient.auth.SessionSnapshot;
import client.privateclient.auth.SessionStatus;
import client.privateclient.events.CoreEventBus;
import client.privateclient.events.EventErrorHandler;
import client.privateclient.logging.SafeLogger;
import client.privateclient.profile.ProfileBridge;
import java.nio.file.Path;
import java.util.Collections;
import java.util.UUID;
import org.apache.logging.log4j.LogManager;
import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;

public class AssociationServiceAuthGateTest {
    @Rule
    public TemporaryFolder temporaryFolder = new TemporaryFolder();

    @Test
    public void registerServerEndpointCollectsMultipleHashes() throws Exception {
        UUID self = UUID.fromString("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        SessionObserver observer = authenticatedObserver(self, "Zinox5");
        AssociationService service = new AssociationService(
                observer,
                new AssociationHttpClient(
                        Collections.singleton("private-client-association.vercel.app")),
                new PeerCache(),
                new SafeLogger(LogManager.getLogger("AssociationServiceAuthGateTest")));

        service.registerServerEndpoint("privatemc.pl", 25565);
        service.registerServerEndpoint("88.99.144.178", 20147);

        assertEquals(2, service.hashesForTest().size());
        assertTrue(service.hashesForTest().contains(ServerHash.of("privatemc.pl", 25565)));
        assertTrue(service.hashesForTest().contains(ServerHash.of("88.99.144.178", 20147)));
        service.shutdown();
    }


    private SessionObserver authenticatedObserver(final UUID uuid, final String username)
            throws Exception {
        Path dataRoot = temporaryFolder.newFolder("data").toPath();
        Path profileFile = dataRoot.resolve("profiles/profile.json");
        ProfileBridge bridge = new ProfileBridge(dataRoot, profileFile);
        SessionProvider provider = new SessionProvider() {
            @Override
            public SessionSnapshot capture() {
                return new SessionSnapshot(
                        username,
                        uuid.toString(),
                        "mojang",
                        true,
                        true);
            }
        };
        SessionObserver observer = new SessionObserver(
                provider,
                new SessionPolicy(),
                bridge,
                new CoreEventBus(new EventErrorHandler() {
                    @Override
                    public void onListenerFailure(
                            client.privateclient.events.CoreEvent event,
                            RuntimeException failure) {
                        // ignore
                    }
                }),
                true);
        observer.refresh();
        assertTrue(observer.getValidation().getStatus() == SessionStatus.AUTHENTICATED);
        return observer;
    }
}
