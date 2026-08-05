package client.privateclient.discord;

import java.io.IOException;
import java.io.InputStream;
import java.util.Properties;

final class DiscordPresenceConfig {
    private static final String APPLICATION_ID_PROPERTY = "privateclient.discord.applicationId";
    private static final String APPLICATION_ID_ENV = "PRIVATE_CLIENT_DISCORD_APPLICATION_ID";
    private static final String LARGE_IMAGE_PROPERTY = "privateclient.discord.largeImage";
    private static final String DEFAULT_LARGE_IMAGE = "private_client";

    private DiscordPresenceConfig() {
    }

    static String applicationId() {
        String value = System.getProperty(APPLICATION_ID_PROPERTY);
        if (value == null || value.trim().isEmpty()) {
            value = System.getenv(APPLICATION_ID_ENV);
        }
        if (value == null || value.trim().isEmpty()) {
            value = bundledApplicationId();
        }
        value = value == null ? "" : value.trim();
        return value.matches("[0-9]{17,20}") ? value : "";
    }

    private static String bundledApplicationId() {
        InputStream stream = DiscordPresenceConfig.class.getResourceAsStream(
                "/discord-presence.properties");
        if (stream == null) {
            return "";
        }
        try {
            Properties properties = new Properties();
            properties.load(stream);
            return properties.getProperty("applicationId", "");
        } catch (IOException ignored) {
            return "";
        } finally {
            try {
                stream.close();
            } catch (IOException ignored) {
            }
        }
    }

    static String largeImage() {
        String value = System.getProperty(LARGE_IMAGE_PROPERTY);
        if (value == null || !value.matches("[a-z0-9_-]{1,64}")) {
            return DEFAULT_LARGE_IMAGE;
        }
        return value;
    }
}
