package client.privateclient.config;

import client.privateclient.modules.impl.NametagsModule;
import client.privateclient.modules.impl.FpsDisplayModule;
import client.privateclient.modules.impl.PerspectiveModule;
import client.privateclient.modules.impl.ToggleSprintModule;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Objects;
import java.util.Set;

public final class CoreConfig {
    public static final int CURRENT_SCHEMA_VERSION = 3;
    public static final Set<String> ALLOWED_MODULE_IDS = Collections.unmodifiableSet(
            new LinkedHashSet<String>(Arrays.asList(
                    ToggleSprintModule.ID,
                    PerspectiveModule.ID,
                    NametagsModule.ID,
                    FpsDisplayModule.ID)));

    private final int schemaVersion;
    private final Map<String, Boolean> modules;
    private final int hudOffsetX;
    private final int hudOffsetY;
    private final double uiScale;
    private final boolean profileBridgeEnabled;
    private final boolean streamerModeEnabled;
    private final boolean discordPresenceEnabled;
    private final LoggingLevel loggingLevel;

    public CoreConfig(
            int schemaVersion,
            Map<String, Boolean> modules,
            int hudOffsetX,
            int hudOffsetY,
            double uiScale,
            boolean profileBridgeEnabled,
            boolean streamerModeEnabled,
            boolean discordPresenceEnabled,
            LoggingLevel loggingLevel) {
        if (schemaVersion != CURRENT_SCHEMA_VERSION) {
            throw new IllegalArgumentException("Unsupported config schema: " + schemaVersion);
        }
        if (hudOffsetX < 0 || hudOffsetX > 4096 || hudOffsetY < 0 || hudOffsetY > 4096) {
            throw new IllegalArgumentException("HUD offsets must be between 0 and 4096");
        }
        if (Double.isNaN(uiScale) || Double.isInfinite(uiScale) || uiScale < 0.75D || uiScale > 2.0D) {
            throw new IllegalArgumentException("UI scale must be between 0.75 and 2.0");
        }
        if (!profileBridgeEnabled) {
            throw new IllegalArgumentException("The authenticated profile bridge cannot be disabled");
        }

        this.schemaVersion = schemaVersion;
        this.modules = normalizeModules(modules);
        this.hudOffsetX = hudOffsetX;
        this.hudOffsetY = hudOffsetY;
        this.uiScale = uiScale;
        this.profileBridgeEnabled = profileBridgeEnabled;
        this.streamerModeEnabled = streamerModeEnabled;
        this.discordPresenceEnabled = discordPresenceEnabled;
        this.loggingLevel = Objects.requireNonNull(loggingLevel, "loggingLevel");
    }

    public static CoreConfig defaults() {
        Map<String, Boolean> modules = new LinkedHashMap<String, Boolean>();
        modules.put(ToggleSprintModule.ID, true);
        modules.put(PerspectiveModule.ID, true);
        modules.put(NametagsModule.ID, true);
        modules.put(FpsDisplayModule.ID, false);
        return new CoreConfig(
                CURRENT_SCHEMA_VERSION,
                modules,
                8,
                8,
                1.0D,
                true,
                false,
                true,
                LoggingLevel.INFO);
    }

    public CoreConfig withModuleStates(Map<String, Boolean> states) {
        return new CoreConfig(
                schemaVersion,
                states,
                hudOffsetX,
                hudOffsetY,
                uiScale,
                profileBridgeEnabled,
                streamerModeEnabled,
                discordPresenceEnabled,
                loggingLevel);
    }

    public CoreConfig withStreamerMode(boolean streamerMode) {
        return new CoreConfig(
                schemaVersion,
                modules,
                hudOffsetX,
                hudOffsetY,
                uiScale,
                profileBridgeEnabled,
                streamerMode,
                discordPresenceEnabled,
                loggingLevel);
    }

    public CoreConfig withDiscordPresence(boolean discordPresence) {
        return new CoreConfig(
                schemaVersion, modules, hudOffsetX, hudOffsetY, uiScale,
                profileBridgeEnabled, streamerModeEnabled, discordPresence, loggingLevel);
    }

    public int getSchemaVersion() {
        return schemaVersion;
    }

    public Map<String, Boolean> getModules() {
        return modules;
    }

    public int getHudOffsetX() {
        return hudOffsetX;
    }

    public int getHudOffsetY() {
        return hudOffsetY;
    }

    public double getUiScale() {
        return uiScale;
    }

    public boolean isProfileBridgeEnabled() {
        return profileBridgeEnabled;
    }

    public boolean isStreamerModeEnabled() {
        return streamerModeEnabled;
    }

    public boolean isDiscordPresenceEnabled() {
        return discordPresenceEnabled;
    }

    public LoggingLevel getLoggingLevel() {
        return loggingLevel;
    }

    private static Map<String, Boolean> normalizeModules(Map<String, Boolean> values) {
        Map<String, Boolean> result = new LinkedHashMap<String, Boolean>();
        Map<String, Boolean> input = values == null
                ? Collections.<String, Boolean>emptyMap()
                : values;
        for (String id : ALLOWED_MODULE_IDS) {
            Boolean enabled = input.get(id);
            boolean defaultState = ToggleSprintModule.ID.equals(id)
                    || PerspectiveModule.ID.equals(id)
                    || NametagsModule.ID.equals(id);
            result.put(id, enabled != null ? enabled.booleanValue() : defaultState);
        }
        return Collections.unmodifiableMap(result);
    }
}
