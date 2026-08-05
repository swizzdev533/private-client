package client.privateclient.discord;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;

final class DiscordIpcFrame {
    static final int HANDSHAKE = 0;
    static final int FRAME = 1;
    static final int CLOSE = 2;
    static final int PING = 3;
    static final int PONG = 4;
    static final int HEADER_BYTES = 8;
    static final int MAX_PAYLOAD_BYTES = 64 * 1024;

    private DiscordIpcFrame() {
    }

    static byte[] encode(int opcode, String json) {
        if (json == null) {
            throw new IllegalArgumentException("JSON payload is required");
        }
        byte[] payload = json.getBytes(StandardCharsets.UTF_8);
        if (payload.length > MAX_PAYLOAD_BYTES) {
            throw new IllegalArgumentException("Discord IPC payload is too large");
        }
        ByteBuffer frame = ByteBuffer.allocate(HEADER_BYTES + payload.length)
                .order(ByteOrder.LITTLE_ENDIAN);
        frame.putInt(opcode);
        frame.putInt(payload.length);
        frame.put(payload);
        return frame.array();
    }

    static int opcode(byte[] header) {
        requireHeader(header);
        return ByteBuffer.wrap(header).order(ByteOrder.LITTLE_ENDIAN).getInt();
    }

    static int payloadLength(byte[] header) {
        requireHeader(header);
        int length = ByteBuffer.wrap(header).order(ByteOrder.LITTLE_ENDIAN).getInt(4);
        if (length < 0 || length > MAX_PAYLOAD_BYTES) {
            throw new IllegalArgumentException("Invalid Discord IPC payload length");
        }
        return length;
    }

    private static void requireHeader(byte[] header) {
        if (header == null || header.length != HEADER_BYTES) {
            throw new IllegalArgumentException("Discord IPC header must contain 8 bytes");
        }
    }
}
