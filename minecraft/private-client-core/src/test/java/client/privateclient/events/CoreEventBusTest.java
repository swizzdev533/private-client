package client.privateclient.events;

import static org.junit.Assert.assertEquals;

import java.util.concurrent.atomic.AtomicInteger;
import org.junit.Test;

public final class CoreEventBusTest {
    @Test
    public void isolatesListenerFailuresAndSupportsUnregister() {
        AtomicInteger failures = new AtomicInteger();
        AtomicInteger delivered = new AtomicInteger();
        CoreEventBus bus = new CoreEventBus((event, failure) -> failures.incrementAndGet());

        Subscription broken = bus.subscribe(CoreEventType.TICK, event -> {
            throw new IllegalStateException("listener failure");
        });
        Subscription healthy = bus.subscribe(CoreEventType.TICK, event -> delivered.incrementAndGet());

        bus.publish(CoreEvent.of(CoreEventType.TICK));
        assertEquals(1, failures.get());
        assertEquals(1, delivered.get());
        assertEquals(2, bus.listenerCount(CoreEventType.TICK));

        broken.close();
        healthy.close();
        assertEquals(0, bus.listenerCount(CoreEventType.TICK));
    }

    @Test
    public void closingBusReleasesListenersAndIgnoresLaterEvents() {
        AtomicInteger delivered = new AtomicInteger();
        CoreEventBus bus = new CoreEventBus((event, failure) -> {
        });
        bus.subscribe(CoreEventType.CLIENT_READY, event -> delivered.incrementAndGet());
        bus.close();

        bus.publish(CoreEvent.of(CoreEventType.CLIENT_READY));

        assertEquals(0, delivered.get());
        assertEquals(0, bus.listenerCount(CoreEventType.CLIENT_READY));
    }
}
