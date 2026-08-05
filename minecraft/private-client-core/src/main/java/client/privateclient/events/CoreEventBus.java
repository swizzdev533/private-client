package client.privateclient.events;

import java.util.EnumMap;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.atomic.AtomicBoolean;

public final class CoreEventBus implements AutoCloseable {
    private final Map<CoreEventType, CopyOnWriteArrayList<CoreEventListener>> listeners =
            new EnumMap<CoreEventType, CopyOnWriteArrayList<CoreEventListener>>(CoreEventType.class);
    private final EventErrorHandler errorHandler;
    private final AtomicBoolean closed = new AtomicBoolean(false);

    public CoreEventBus(EventErrorHandler errorHandler) {
        this.errorHandler = Objects.requireNonNull(errorHandler, "errorHandler");
        for (CoreEventType type : CoreEventType.values()) {
            listeners.put(type, new CopyOnWriteArrayList<CoreEventListener>());
        }
    }

    public Subscription subscribe(CoreEventType type, CoreEventListener listener) {
        Objects.requireNonNull(type, "type");
        Objects.requireNonNull(listener, "listener");
        if (closed.get()) {
            throw new IllegalStateException("Event bus is closed");
        }
        CopyOnWriteArrayList<CoreEventListener> bucket = listeners.get(type);
        bucket.add(listener);
        return new ListenerSubscription(bucket, listener);
    }

    public void publish(CoreEvent event) {
        Objects.requireNonNull(event, "event");
        if (closed.get()) {
            return;
        }
        for (CoreEventListener listener : listeners.get(event.getType())) {
            try {
                listener.onEvent(event);
            } catch (RuntimeException failure) {
                errorHandler.onListenerFailure(event, failure);
            }
        }
    }

    public int listenerCount(CoreEventType type) {
        Objects.requireNonNull(type, "type");
        return listeners.get(type).size();
    }

    @Override
    public void close() {
        if (closed.compareAndSet(false, true)) {
            for (CopyOnWriteArrayList<CoreEventListener> bucket : listeners.values()) {
                bucket.clear();
            }
        }
    }

    private static final class ListenerSubscription implements Subscription {
        private final CopyOnWriteArrayList<CoreEventListener> bucket;
        private final CoreEventListener listener;
        private final AtomicBoolean closed = new AtomicBoolean(false);

        private ListenerSubscription(
                CopyOnWriteArrayList<CoreEventListener> bucket,
                CoreEventListener listener) {
            this.bucket = bucket;
            this.listener = listener;
        }

        @Override
        public void close() {
            if (closed.compareAndSet(false, true)) {
                bucket.remove(listener);
            }
        }
    }
}
