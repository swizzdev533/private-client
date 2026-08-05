package client.privateclient.discord;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

import java.io.Closeable;
import java.io.IOException;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.Test;

public final class DiscordDaemonThreadsTest {
    @Test
    public void blockingCloseReturnsImmediatelyAndRunsOnDaemon() throws Exception {
        final CountDownLatch closeEntered = new CountDownLatch(1);
        final CountDownLatch releaseClose = new CountDownLatch(1);
        final CountDownLatch callerReturned = new CountDownLatch(1);
        final AtomicReference<Thread> closerReference = new AtomicReference<Thread>();
        Closeable blockingResource = new Closeable() {
            @Override
            public void close() throws IOException {
                closeEntered.countDown();
                try {
                    releaseClose.await();
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    throw new IOException("Interrupted while closing", interrupted);
                }
            }
        };

        Thread caller = DiscordDaemonThreads.newThread("discord-close-test-caller", new Runnable() {
            @Override
            public void run() {
                closerReference.set(DiscordDaemonThreads.closeAsync(blockingResource));
                callerReturned.countDown();
            }
        });
        caller.start();

        Thread closer = null;
        try {
            assertTrue("Scheduling close must not wait for Closeable.close()",
                    callerReturned.await(1L, TimeUnit.SECONDS));
            closer = closerReference.get();
            assertNotNull(closer);
            assertTrue("The Discord worker helper must create daemon threads", caller.isDaemon());
            assertTrue("The Discord closer must be a daemon thread", closer.isDaemon());
            assertTrue("The close operation was not started", closeEntered.await(1L, TimeUnit.SECONDS));
            assertTrue("The closer should still be blocked in the test resource", closer.isAlive());
        } finally {
            releaseClose.countDown();
        }

        closer.join(1_000L);
        assertFalse("The closer did not finish after the resource was released", closer.isAlive());
    }
}
