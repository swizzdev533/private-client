package client.privateclient.util;

import java.util.Collection;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class StreamerModeUtil {
    private static final Pattern SERVER_PLAYER_EVENT_NAME = Pattern.compile(
            "(?i)(?<![a-zA-Z0-9_])([a-zA-Z0-9_]{3,16})"
                    + "(?=\\s+(?:dołącza|dolacza|dołączył|dolaczyl|opuszcza|opuścił|opuscil|"
                    + "joined|left|has joined|has left)\\b)");

    private StreamerModeUtil() {
    }

    public static String sanitizeName(String name, boolean enabled) {
        if (!enabled || name == null || name.trim().isEmpty()) {
            return name;
        }
        return "???";
    }

    public static String playerAlias(int index) {
        if (index < 1) {
            throw new IllegalArgumentException("Player alias index must be positive");
        }
        return "Player " + index;
    }

    public static String sanitizeText(String text, boolean enabled) {
        if (!enabled || text == null || text.trim().isEmpty()) {
            return text;
        }
        return text.replaceAll("<[a-zA-Z0-9_]{3,16}>", "<???>")
                   .replaceAll("^([a-zA-Z0-9_]{3,16}):", "???:");
    }

    public static String sanitizeKnownNames(
            String text,
            boolean enabled,
            Collection<String> playerNames) {
        String sanitized = sanitizeText(text, enabled);
        if (!enabled || sanitized == null || playerNames == null) {
            return sanitized;
        }
        for (String playerName : playerNames) {
            if (playerName == null || !playerName.matches("[a-zA-Z0-9_]{3,16}")) {
                continue;
            }
            Pattern exactName = Pattern.compile(
                    "(?i)(?<![a-zA-Z0-9_])" + Pattern.quote(playerName) + "(?![a-zA-Z0-9_])");
            sanitized = exactName.matcher(sanitized).replaceAll(Matcher.quoteReplacement("???"));
        }
        return SERVER_PLAYER_EVENT_NAME.matcher(sanitized).replaceAll(Matcher.quoteReplacement("???"));
    }
}
