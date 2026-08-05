package client.privateclient.srv;

public final class ResolvedServerAddress {
    private final String host;
    private final int port;

    ResolvedServerAddress(String host, int port) {
        this.host = host;
        this.port = port;
    }

    public String getHost() {
        return host;
    }

    public int getPort() {
        return port;
    }
}
