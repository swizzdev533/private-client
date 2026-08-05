package client.privateclient.discord;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.nio.charset.StandardCharsets;
import org.junit.Test;

public final class DiscordIpcFrameTest {
    @Test
    public void encodesLittleEndianHeaderAndUtf8Payload() {
        byte[] frame = DiscordIpcFrame.encode(DiscordIpcFrame.HANDSHAKE, "{\"v\":1}");
        byte[] header = new byte[DiscordIpcFrame.HEADER_BYTES];
        System.arraycopy(frame, 0, header, 0, header.length);

        assertEquals(DiscordIpcFrame.HANDSHAKE, DiscordIpcFrame.opcode(header));
        assertEquals(frame.length - header.length, DiscordIpcFrame.payloadLength(header));
        assertEquals("{\"v\":1}", new String(
                frame, header.length, frame.length - header.length, StandardCharsets.UTF_8));
    }

    @Test(expected = IllegalArgumentException.class)
    public void rejectsOversizedPayloads() {
        StringBuilder payload = new StringBuilder();
        for (int index = 0; index <= DiscordIpcFrame.MAX_PAYLOAD_BYTES; index++) {
            payload.append('x');
        }
        DiscordIpcFrame.encode(DiscordIpcFrame.FRAME, payload.toString());
    }

    @Test
    public void activityExcludesPlayerAndServerData() {
        String payload = DiscordPresencePayload.activity(42L, 100L, "private_client");
        assertTrue(payload.contains("Playing Private Client"));
        assertTrue(payload.contains("Minecraft 1.8.9"));
        assertFalse(payload.contains("username"));
        assertFalse(payload.contains("server"));
        assertFalse(payload.contains("uuid"));
    }
}
