package client.privateclient.association;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.util.Collections;
import java.util.HashSet;
import java.util.Set;
import java.util.UUID;
import org.junit.Test;

public class PeerCacheTest {
    @Test
    public void retainsPeersInsideSoftWindow() {
        PeerCache cache = new PeerCache(60_000L);
        UUID peer = UUID.fromString("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        Set<UUID> peers = new HashSet<UUID>();
        peers.add(peer);
        cache.replaceAll(peers, 1_000L);
        assertTrue(cache.isAssociated(peer, 30_000L));
    }

    @Test
    public void expiresPeersOutsideSoftWindow() {
        PeerCache cache = new PeerCache(60_000L);
        UUID peer = UUID.fromString("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        cache.replaceAll(Collections.singleton(peer), 1_000L);
        assertFalse(cache.isAssociated(peer, 70_000L));
    }

    @Test
    public void mergeAddsPeerWithoutClearingOthers() {
        PeerCache cache = new PeerCache(60_000L);
        UUID a = UUID.fromString("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        UUID b = UUID.fromString("11111111-2222-3333-4444-555555555555");
        cache.replaceAll(Collections.singleton(a), 1_000L);
        cache.merge(b, 2_000L);
        assertTrue(cache.isAssociated(a, 3_000L));
        assertTrue(cache.isAssociated(b, 3_000L));
    }

    @Test
    public void associatesByUsernameWhenUuidDiffers() {
        PeerCache cache = new PeerCache(60_000L);
        UUID account = UUID.fromString("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        UUID offlineEntity = UUID.fromString("11111111-2222-3333-4444-555555555555");
        Set<UUID> peers = new HashSet<UUID>();
        peers.add(account);
        Set<String> names = new HashSet<String>();
        names.add("Zinox5");
        cache.replaceAll(peers, names, 1_000L);
        assertFalse(cache.isAssociated(offlineEntity, 2_000L));
        assertTrue(cache.isAssociated(offlineEntity, "zinox5", 2_000L));
        assertTrue(cache.isAssociated(null, "Zinox5", 2_000L));
    }
}
