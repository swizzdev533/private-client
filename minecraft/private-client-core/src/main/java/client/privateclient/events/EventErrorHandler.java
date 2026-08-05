package client.privateclient.events;

public interface EventErrorHandler {
    void onListenerFailure(CoreEvent event, RuntimeException failure);
}
