package client.privateclient.discord;

import java.io.Closeable;
import java.io.IOException;
import java.util.concurrent.atomic.AtomicLong;

final class DiscordDaemonThreads {
    private static final AtomicLong CLOSE_THREAD_SEQUENCE = new AtomicLong();

    private DiscordDaemonThreads() {
    }

    static Thread newThread(String name, Runnable task) {
        Thread thread = new Thread(task, name);
        thread.setDaemon(true);
        return thread;
    }

    static Thread closeAsync(final Closeable resource) {
        Thread closer = newThread(
                "private-client-discord-rpc-close-" + CLOSE_THREAD_SEQUENCE.incrementAndGet(),
                new Runnable() {
                    @Override
                    public void run() {
                        try {
                            resource.close();
                        } catch (IOException ignored) {
                        }
                    }
                });
        closer.start();
        return closer;
    }
}
