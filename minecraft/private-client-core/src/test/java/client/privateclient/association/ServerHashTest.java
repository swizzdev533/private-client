package client.privateclient.association;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotEquals;

import org.junit.Test;

public class ServerHashTest {
    @Test
    public void hashesAreStableAndCaseInsensitiveForHost() {
        String lower = ServerHash.of("Play.Hypixel.Net", 25565);
        String upper = ServerHash.of("play.hypixel.net", 25565);
        assertEquals(64, lower.length());
        assertEquals(lower, upper);
    }

    @Test
    public void differentPortsProduceDifferentHashes() {
        assertNotEquals(
                ServerHash.of("play.example.net", 25565),
                ServerHash.of("play.example.net", 25566));
    }
}
