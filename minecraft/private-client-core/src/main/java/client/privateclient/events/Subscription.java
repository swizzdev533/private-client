package client.privateclient.events;

public interface Subscription extends AutoCloseable {
    @Override
    void close();
}
