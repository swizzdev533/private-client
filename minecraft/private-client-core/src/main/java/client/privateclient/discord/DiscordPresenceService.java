package client.privateclient.discord;

import client.privateclient.logging.SafeLogger;
import java.io.EOFException;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.lang.management.ManagementFactory;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.util.concurrent.atomic.AtomicBoolean;

public final class DiscordPresenceService {
    private static final long RETRY_MILLIS = 15_000L;
    private static final int PIPE_CANDIDATES = 10;

    private final SafeLogger log;
    private final AtomicBoolean running = new AtomicBoolean(false);
    private final long startedAtSeconds = Instant.now().getEpochSecond();
    private volatile boolean enabled;
    private volatile RandomAccessFile connection;
    private Thread worker;

    public DiscordPresenceService(SafeLogger log) {
        if (log == null) {
            throw new IllegalArgumentException("Logger is required");
        }
        this.log = log;
    }

    public synchronized void start(boolean initiallyEnabled) {
        enabled = initiallyEnabled;
        if (!running.compareAndSet(false, true)) {
            return;
        }
        worker = DiscordDaemonThreads.newThread("private-client-discord-rpc", new Runnable() {
            @Override
            public void run() {
                runLoop();
            }
        });
        worker.start();
    }

    public synchronized void setEnabled(boolean nextEnabled) {
        enabled = nextEnabled;
        if (!nextEnabled) {
            closeConnection();
        }
        notifyAll();
    }

    public synchronized void stop() {
        enabled = false;
        running.set(false);
        closeConnection();
        notifyAll();
        if (worker != null) {
            worker.interrupt();
        }
    }

    private void runLoop() {
        String applicationId = DiscordPresenceConfig.applicationId();
        if (applicationId.isEmpty()) {
            log.warn("Discord Status is unavailable: configure PRIVATE_CLIENT_DISCORD_APPLICATION_ID");
            return;
        }
        if (!System.getProperty("os.name", "").toLowerCase().contains("windows")) {
            log.warn("Discord Status currently supports the Windows Discord desktop client only");
            return;
        }

        while (running.get()) {
            if (!enabled) {
                pauseUntilRetry();
                continue;
            }
            try {
                connectAndPublish(applicationId);
                waitForConnection();
            } catch (IOException exception) {
                closeConnection();
                pauseUntilRetry();
            }
        }
    }

    private void connectAndPublish(String applicationId) throws IOException {
        RandomAccessFile pipe = openPipe();
        if (!registerConnection(pipe)) {
            DiscordDaemonThreads.closeAsync(pipe);
            return;
        }
        write(pipe, DiscordIpcFrame.HANDSHAKE, DiscordPresencePayload.handshake(applicationId));
        readFrame(pipe);
        write(pipe, DiscordIpcFrame.FRAME, DiscordPresencePayload.activity(
                currentProcessId(), startedAtSeconds, DiscordPresenceConfig.largeImage()));
        log.info("Discord Status connected");
    }

    private void waitForConnection() throws IOException {
        while (running.get() && enabled) {
            RandomAccessFile pipe = connection;
            if (pipe == null) {
                return;
            }
            Frame frame = readFrame(pipe);
            if (frame.opcode == DiscordIpcFrame.PING) {
                write(pipe, DiscordIpcFrame.PONG, frame.json);
            } else if (frame.opcode == DiscordIpcFrame.CLOSE) {
                throw new EOFException("Discord closed the IPC session");
            }
        }
    }

    private static RandomAccessFile openPipe() throws IOException {
        IOException lastFailure = null;
        for (int index = 0; index < PIPE_CANDIDATES; index++) {
            try {
                return new RandomAccessFile("\\\\.\\pipe\\discord-ipc-" + index, "rw");
            } catch (IOException failure) {
                lastFailure = failure;
            }
        }
        throw lastFailure == null ? new IOException("Discord IPC pipe was not found") : lastFailure;
    }

    private static void write(RandomAccessFile pipe, int opcode, String json) throws IOException {
        pipe.write(DiscordIpcFrame.encode(opcode, json));
    }

    private static Frame readFrame(RandomAccessFile pipe) throws IOException {
        byte[] header = new byte[DiscordIpcFrame.HEADER_BYTES];
        pipe.readFully(header);
        int length;
        try {
            length = DiscordIpcFrame.payloadLength(header);
        } catch (IllegalArgumentException invalidFrame) {
            throw new IOException("Discord sent an invalid IPC frame", invalidFrame);
        }
        byte[] payload = new byte[length];
        pipe.readFully(payload);
        return new Frame(DiscordIpcFrame.opcode(header), new String(payload, StandardCharsets.UTF_8));
    }

    private static long currentProcessId() {
        String runtimeName = ManagementFactory.getRuntimeMXBean().getName();
        int separator = runtimeName.indexOf('@');
        String value = separator < 0 ? runtimeName : runtimeName.substring(0, separator);
        try {
            return Long.parseLong(value);
        } catch (NumberFormatException ignored) {
            return 0L;
        }
    }

    private synchronized void pauseUntilRetry() {
        try {
            wait(RETRY_MILLIS);
        } catch (InterruptedException ignored) {
            Thread.currentThread().interrupt();
        }
    }

    private synchronized boolean registerConnection(RandomAccessFile pipe) {
        if (!running.get() || !enabled) {
            return false;
        }
        connection = pipe;
        return true;
    }

    private void closeConnection() {
        RandomAccessFile pipe;
        synchronized (this) {
            pipe = connection;
            connection = null;
        }
        if (pipe != null) {
            // Windows can block close() while the daemon worker is inside readFully().
            // Keep shutdown and settings callers independent from that native I/O wait.
            DiscordDaemonThreads.closeAsync(pipe);
        }
    }

    private static final class Frame {
        private final int opcode;
        private final String json;

        private Frame(int opcode, String json) {
            this.opcode = opcode;
            this.json = json;
        }
    }
}
