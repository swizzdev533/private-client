package client.privateclient.association.net;

import client.privateclient.association.AssociationEndpoints;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import javax.net.ssl.HttpsURLConnection;

/**
 * Bounded HTTPS client for the association presence API. Only the pinned host
 * is accepted; redirects off-host are rejected.
 */
public final class AssociationHttpClient {
    public static final int CONNECT_TIMEOUT_MS = 4_000;
    public static final int READ_TIMEOUT_MS = 6_000;
    public static final int MAX_BODY_BYTES = 16_384;
    public static final int MAX_REDIRECTS = 3;

    private final Set<String> allowedHosts;

    public AssociationHttpClient() {
        this(Collections.singleton(AssociationEndpoints.HOST));
    }

    public AssociationHttpClient(Set<String> allowedHosts) {
        if (allowedHosts == null || allowedHosts.isEmpty()) {
            throw new IllegalArgumentException("allowedHosts required");
        }
        Set<String> normalized = new HashSet<String>();
        for (String host : allowedHosts) {
            normalized.add(host.toLowerCase(Locale.ROOT));
        }
        this.allowedHosts = Collections.unmodifiableSet(normalized);
    }

    public List<UUID> heartbeat(String jsonBody) throws IOException {
        return new ArrayList<UUID>(heartbeatPeers(AssociationEndpoints.PRESENCE_URL, jsonBody).keySet());
    }

    public List<UUID> heartbeat(String url, String jsonBody) throws IOException {
        return new ArrayList<UUID>(heartbeatPeers(url, jsonBody).keySet());
    }

    public Map<UUID, String> heartbeatPeers(String jsonBody) throws IOException {
        return heartbeatPeers(AssociationEndpoints.PRESENCE_URL, jsonBody);
    }

    public Map<UUID, String> heartbeatPeers(String url, String jsonBody) throws IOException {
        if (jsonBody == null || jsonBody.isEmpty()) {
            throw new IllegalArgumentException("jsonBody required");
        }
        if (jsonBody.getBytes(StandardCharsets.UTF_8).length > MAX_BODY_BYTES) {
            throw new IOException("Request body too large");
        }

        String current = url;
        for (int redirect = 0; redirect <= MAX_REDIRECTS; redirect++) {
            URL parsed = new URL(current);
            assertAllowed(parsed);

            HttpURLConnection connection = (HttpURLConnection) parsed.openConnection();
            try {
                connection.setInstanceFollowRedirects(false);
                connection.setConnectTimeout(CONNECT_TIMEOUT_MS);
                connection.setReadTimeout(READ_TIMEOUT_MS);
                connection.setRequestMethod("POST");
                connection.setDoOutput(true);
                connection.setRequestProperty("Content-Type", "application/json; charset=utf-8");
                connection.setRequestProperty("Accept", "application/json");
                connection.setRequestProperty("User-Agent", "PrivateClient/" + AssociationEndpoints.CLIENT_VERSION);
                if (connection instanceof HttpsURLConnection) {
                    // default JVM trust store
                } else if ("https".equalsIgnoreCase(parsed.getProtocol())) {
                    throw new IOException("HTTPS required");
                } else {
                    throw new IOException("Plain HTTP is not allowed");
                }

                byte[] payload = jsonBody.getBytes(StandardCharsets.UTF_8);
                connection.setFixedLengthStreamingMode(payload.length);
                OutputStream output = connection.getOutputStream();
                try {
                    output.write(payload);
                } finally {
                    output.close();
                }

                int status = connection.getResponseCode();
                if (status >= 300 && status < 400) {
                    String location = connection.getHeaderField("Location");
                    if (location == null || location.isEmpty()) {
                        throw new IOException("Redirect without Location");
                    }
                    current = new URL(parsed, location).toExternalForm();
                    continue;
                }

                InputStream stream = status >= 400
                        ? connection.getErrorStream()
                        : connection.getInputStream();
                String body = readBounded(stream);
                if (status < 200 || status >= 300) {
                    throw new IOException("Presence HTTP " + status);
                }
                return PresencePayload.parsePeers(body);
            } finally {
                connection.disconnect();
            }
        }
        throw new IOException("Too many redirects");
    }

    private void assertAllowed(URL url) throws IOException {
        if (!"https".equalsIgnoreCase(url.getProtocol())) {
            throw new IOException("HTTPS required");
        }
        String host = url.getHost();
        if (host == null || !allowedHosts.contains(host.toLowerCase(Locale.ROOT))) {
            throw new IOException("Host not allowlisted");
        }
        int port = url.getPort();
        if (port != -1 && port != 443) {
            throw new IOException("Unexpected port");
        }
        if (url.getUserInfo() != null) {
            throw new IOException("Userinfo not allowed");
        }
    }

    private static String readBounded(InputStream stream) throws IOException {
        if (stream == null) {
            return "";
        }
        ByteArrayOutputStream buffer = new ByteArrayOutputStream();
        byte[] chunk = new byte[1024];
        int total = 0;
        int read;
        while ((read = stream.read(chunk)) != -1) {
            total += read;
            if (total > MAX_BODY_BYTES) {
                throw new IOException("Response body too large");
            }
            buffer.write(chunk, 0, read);
        }
        return new String(buffer.toByteArray(), StandardCharsets.UTF_8);
    }
}
