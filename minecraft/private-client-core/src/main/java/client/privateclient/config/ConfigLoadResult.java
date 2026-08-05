package client.privateclient.config;

import java.nio.file.Path;
import java.util.Optional;

public final class ConfigLoadResult {
    public enum Status {
        LOADED,
        CREATED_DEFAULTS,
        MIGRATED,
        RECOVERED_CORRUPT
    }

    private final CoreConfig config;
    private final Status status;
    private final Path quarantinedFile;

    public ConfigLoadResult(CoreConfig config, Status status, Path quarantinedFile) {
        if (config == null || status == null) {
            throw new IllegalArgumentException("Config and status are required");
        }
        this.config = config;
        this.status = status;
        this.quarantinedFile = quarantinedFile;
    }

    public CoreConfig getConfig() {
        return config;
    }

    public Status getStatus() {
        return status;
    }

    public Optional<Path> getQuarantinedFile() {
        return Optional.ofNullable(quarantinedFile);
    }
}
