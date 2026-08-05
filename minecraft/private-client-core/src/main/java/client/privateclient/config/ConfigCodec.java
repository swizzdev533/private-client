package client.privateclient.config;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.util.LinkedHashMap;
import java.util.Map;

public final class ConfigCodec {
    private final Gson gson = new GsonBuilder()
            .disableHtmlEscaping()
            .setPrettyPrinting()
            .create();

    public CoreConfig decode(String json) {
        if (json == null || json.trim().isEmpty()) {
            throw new IllegalArgumentException("Configuration is empty");
        }
        JsonElement rootElement = new JsonParser().parse(json);
        if (!rootElement.isJsonObject()) {
            throw new IllegalArgumentException("Configuration root must be an object");
        }
        JsonObject root = rootElement.getAsJsonObject();
        int schemaVersion = integer(root, "schemaVersion", 1);
        if (schemaVersion < 1 || schemaVersion > CoreConfig.CURRENT_SCHEMA_VERSION) {
            throw new IllegalArgumentException("Unsupported configuration schema: " + schemaVersion);
        }

        Map<String, Boolean> modules = schemaVersion == 1
                ? decodeVersionOneModules(root)
                : decodeVersionTwoModules(root);

        JsonObject hud = object(root, "hudLayout");
        int offsetX = hud == null ? 8 : integer(hud, "offsetX", 8);
        int offsetY = hud == null ? 8 : integer(hud, "offsetY", 8);
        double uiScale = decimal(root, "uiScale", 1.0D);
        boolean profileBridge = bool(root, "profileBridge", true);
        boolean streamerMode = bool(root, "streamerMode", false);
        boolean discordPresence = bool(root, "discordPresence", true);
        LoggingLevel loggingLevel = enumValue(
                root,
                "loggingLevel",
                LoggingLevel.class,
                LoggingLevel.INFO);

        return new CoreConfig(
                CoreConfig.CURRENT_SCHEMA_VERSION,
                modules,
                offsetX,
                offsetY,
                uiScale,
                profileBridge,
                streamerMode,
                discordPresence,
                loggingLevel);
    }

    public String encode(CoreConfig config) {
        JsonObject root = new JsonObject();
        root.addProperty("schemaVersion", config.getSchemaVersion());

        JsonObject modules = new JsonObject();
        for (Map.Entry<String, Boolean> entry : config.getModules().entrySet()) {
            modules.addProperty(entry.getKey(), entry.getValue());
        }
        root.add("modules", modules);

        JsonObject hud = new JsonObject();
        hud.addProperty("offsetX", config.getHudOffsetX());
        hud.addProperty("offsetY", config.getHudOffsetY());
        root.add("hudLayout", hud);
        root.addProperty("uiScale", config.getUiScale());
        root.addProperty("profileBridge", config.isProfileBridgeEnabled());
        root.addProperty("streamerMode", config.isStreamerModeEnabled());
        root.addProperty("discordPresence", config.isDiscordPresenceEnabled());
        root.addProperty("loggingLevel", config.getLoggingLevel().name());
        return gson.toJson(root) + System.lineSeparator();
    }

    public int readSchemaVersion(String json) {
        JsonElement rootElement = new JsonParser().parse(json);
        if (!rootElement.isJsonObject()) {
            throw new IllegalArgumentException("Configuration root must be an object");
        }
        return integer(rootElement.getAsJsonObject(), "schemaVersion", 1);
    }

    private static Map<String, Boolean> decodeVersionOneModules(JsonObject root) {
        Map<String, Boolean> result = disabledModules();
        JsonElement enabled = root.get("enabledModules");
        if (enabled != null && enabled.isJsonArray()) {
            for (JsonElement element : enabled.getAsJsonArray()) {
                if (element.isJsonPrimitive()) {
                    String id = element.getAsString();
                    if (CoreConfig.ALLOWED_MODULE_IDS.contains(id)) {
                        result.put(id, true);
                    }
                }
            }
            return result;
        }
        return decodeVersionTwoModules(root);
    }

    private static Map<String, Boolean> decodeVersionTwoModules(JsonObject root) {
        Map<String, Boolean> result = defaults();
        JsonObject modules = object(root, "modules");
        if (modules == null) {
            return result;
        }
        for (String id : CoreConfig.ALLOWED_MODULE_IDS) {
            JsonElement value = modules.get(id);
            if (value != null && value.isJsonPrimitive()) {
                result.put(id, value.getAsBoolean());
            }
        }
        return result;
    }

    private static Map<String, Boolean> defaults() {
        return new LinkedHashMap<String, Boolean>(CoreConfig.defaults().getModules());
    }

    private static Map<String, Boolean> disabledModules() {
        Map<String, Boolean> result = new LinkedHashMap<String, Boolean>();
        for (String id : CoreConfig.ALLOWED_MODULE_IDS) {
            result.put(id, false);
        }
        return result;
    }

    private static JsonObject object(JsonObject root, String name) {
        JsonElement value = root.get(name);
        if (value == null || value.isJsonNull()) {
            return null;
        }
        if (!value.isJsonObject()) {
            throw new IllegalArgumentException(name + " must be an object");
        }
        return value.getAsJsonObject();
    }

    private static int integer(JsonObject root, String name, int defaultValue) {
        JsonElement value = root.get(name);
        if (value == null || value.isJsonNull()) {
            return defaultValue;
        }
        return value.getAsInt();
    }

    private static double decimal(JsonObject root, String name, double defaultValue) {
        JsonElement value = root.get(name);
        if (value == null || value.isJsonNull()) {
            return defaultValue;
        }
        return value.getAsDouble();
    }

    private static boolean bool(JsonObject root, String name, boolean defaultValue) {
        JsonElement value = root.get(name);
        if (value == null || value.isJsonNull()) {
            return defaultValue;
        }
        return value.getAsBoolean();
    }

    private static <E extends Enum<E>> E enumValue(
            JsonObject root,
            String name,
            Class<E> enumClass,
            E defaultValue) {
        JsonElement value = root.get(name);
        if (value == null || value.isJsonNull() || !value.isJsonPrimitive()) {
            return defaultValue;
        }
        try {
            return Enum.valueOf(enumClass, value.getAsString());
        } catch (IllegalArgumentException unmapped) {
            return defaultValue;
        }
    }
}
