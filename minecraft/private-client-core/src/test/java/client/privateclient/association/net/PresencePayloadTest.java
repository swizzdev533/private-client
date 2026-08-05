package client.privateclient.association.net;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.util.List;
import java.util.UUID;
import org.junit.Test;

public class PresencePayloadTest {
    @Test
    public void buildsAndParsesPeerList() {
        UUID self = UUID.fromString("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        StringBuilder hash = new StringBuilder(64);
        for (int i = 0; i < 32; i++) {
            hash.append("ab");
        }
        String body = PresencePayload.buildRequestJson(
                self,
                "Zinox5",
                hash.toString(),
                "1.0.0");
        assertTrue(body.contains("\"schemaVersion\":1"));
        assertTrue(body.contains("\"username\":\"Zinox5\""));

        List<UUID> peers = PresencePayload.parsePeerUuids(
                "{\"schemaVersion\":1,\"peers\":[\"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee\","
                        + "\"11111111-2222-3333-4444-555555555555\"]}");
        assertEquals(2, peers.size());
        assertTrue(peers.contains(self));

        java.util.Map<UUID, String> entries = PresencePayload.parsePeers(
                "{\"schemaVersion\":1,\"peers\":[\"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee\"],"
                        + "\"peerEntries\":[{\"uuid\":\"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee\","
                        + "\"username\":\"Zinox5\"},{\"uuid\":\"11111111-2222-3333-4444-555555555555\","
                        + "\"username\":\"Swizz5\"}]}");
        assertEquals(2, entries.size());
        assertEquals("zinox5", entries.get(self));
        assertEquals(
                "swizz5",
                entries.get(UUID.fromString("11111111-2222-3333-4444-555555555555")));
    }
}
