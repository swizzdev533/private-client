package client.privateclient.security;

import java.nio.file.Path;
import java.nio.file.Paths;

public final class CorePaths {
    public static final String DATA_DIR_PROPERTY = "privateclient.dataDir";

    private final Path dataRoot;

    private CorePaths(Path dataRoot) {
        this.dataRoot = SafePaths.normalizeRoot(dataRoot);
    }

    public static CorePaths discover() {
        String override = trimToNull(System.getProperty(DATA_DIR_PROPERTY));
        if (override != null) {
            return new CorePaths(Paths.get(override));
        }

        String appData = trimToNull(System.getenv("APPDATA"));
        if (appData != null) {
            return new CorePaths(Paths.get(appData, "PrivateClient"));
        }

        String userHome = trimToNull(System.getProperty("user.home"));
        if (userHome == null) {
            throw new IllegalStateException("Neither APPDATA nor user.home is available");
        }
        return new CorePaths(Paths.get(userHome, "AppData", "Roaming", "PrivateClient"));
    }

    public Path getDataRoot() {
        return dataRoot;
    }

    public Path getConfigFile() {
        return SafePaths.resolveRelative(dataRoot, "core/config.json");
    }

    public Path getProfileFile() {
        return SafePaths.resolveRelative(dataRoot, "profiles/profile.json");
    }

    private static String trimToNull(String value) {
        if (value == null || value.trim().isEmpty()) {
            return null;
        }
        return value.trim();
    }
}
