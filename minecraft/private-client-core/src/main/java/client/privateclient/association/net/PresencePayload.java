package client.privateclient.association.net;

import client.privateclient.association.AssociationEndpoints;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class PresencePayload {
    private static final Pattern UUID_PATTERN = Pattern.compile(
            "[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}");
    private static final Pattern ENTRY_PATTERN = Pattern.compile(
            "\\{\\s*\"uuid\"\\s*:\\s*\"([0-9a-fA-F-]{36})\"\\s*(?:,\\s*\"username\"\\s*:\\s*\"([A-Za-z0-9_]{1,16})\")?\\s*\\}");

    private PresencePayload() {
    }

    public static String buildRequestJson(
            UUID uuid,
            String username,
            String serverHash,
            String clientVersion) {
        if (uuid == null) {
            throw new IllegalArgumentException("uuid is required");
        }
        if (username == null || !username.matches("^[A-Za-z0-9_]{1,16}$")) {
            throw new IllegalArgumentException("username invalid");
        }
        if (serverHash == null || !serverHash.matches("^[0-9a-f]{64}$")) {
            throw new IllegalArgumentException("serverHash invalid");
        }
        if (clientVersion == null || clientVersion.isEmpty() || clientVersion.length() > 32) {
            throw new IllegalArgumentException("clientVersion invalid");
        }
        return "{"
                + "\"schemaVersion\":" + AssociationEndpoints.SCHEMA_VERSION + ","
                + "\"uuid\":\"" + uuid.toString().toLowerCase(Locale.ROOT) + "\","
                + "\"username\":\"" + username + "\","
                + "\"serverHash\":\"" + serverHash + "\","
                + "\"clientVersion\":\"" + escape(clientVersion) + "\""
                + "}";
    }

    public static List<UUID> parsePeerUuids(String json) {
        return new ArrayList<UUID>(parsePeers(json).keySet());
    }

    /**
     * @return map of peer UUID → lowercase username (empty string when unknown)
     */
    public static Map<UUID, String> parsePeers(String json) {
        if (json == null || json.isEmpty()) {
            return Collections.emptyMap();
        }
        Map<UUID, String> peers = new LinkedHashMap<UUID, String>();

        int entriesIndex = json.indexOf("\"peerEntries\"");
        if (entriesIndex >= 0) {
            int arrayStart = json.indexOf('[', entriesIndex);
            int arrayEnd = json.indexOf(']', arrayStart);
            if (arrayStart >= 0 && arrayEnd > arrayStart) {
                String array = json.substring(arrayStart, arrayEnd + 1);
                Matcher entryMatcher = ENTRY_PATTERN.matcher(array);
                while (entryMatcher.find()) {
                    try {
                        UUID uuid = UUID.fromString(entryMatcher.group(1).toLowerCase(Locale.ROOT));
                        String username = entryMatcher.group(2);
                        peers.put(
                                uuid,
                                username == null ? "" : username.toLowerCase(Locale.ROOT));
                    } catch (IllegalArgumentException ignored) {
                        // skip malformed
                    }
                }
            }
        }

        if (!peers.isEmpty()) {
            return peers;
        }

        int peersIndex = json.indexOf("\"peers\"");
        if (peersIndex < 0) {
            return Collections.emptyMap();
        }
        int arrayStart = json.indexOf('[', peersIndex);
        int arrayEnd = json.indexOf(']', arrayStart);
        if (arrayStart < 0 || arrayEnd < 0) {
            return Collections.emptyMap();
        }
        String array = json.substring(arrayStart, arrayEnd + 1);
        Matcher matcher = UUID_PATTERN.matcher(array);
        while (matcher.find()) {
            try {
                UUID uuid = UUID.fromString(matcher.group().toLowerCase(Locale.ROOT));
                if (!peers.containsKey(uuid)) {
                    peers.put(uuid, "");
                }
            } catch (IllegalArgumentException ignored) {
                // skip malformed
            }
        }
        return peers;
    }

    private static String escape(String value) {
        return value.replace("\\", "\\\\").replace("\"", "\\\"");
    }
}
