package client.privateclient.config;

import client.privateclient.security.AtomicFiles;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.time.Clock;

public final class ConfigStore {
    private static final long MAX_CONFIG_BYTES = 1024L * 1024L;

    private final Path configFile;
    private final ConfigCodec codec;
    private final Clock clock;

    public ConfigStore(Path configFile) {
        this(configFile, new ConfigCodec(), Clock.systemUTC());
    }

    ConfigStore(Path configFile, ConfigCodec codec, Clock clock) {
        if (configFile == null || codec == null || clock == null) {
            throw new IllegalArgumentException("Config file, codec and clock are required");
        }
        this.configFile = configFile.toAbsolutePath().normalize();
        this.codec = codec;
        this.clock = clock;
    }

    public synchronized ConfigLoadResult load() throws IOException {
        if (!Files.exists(configFile, LinkOption.NOFOLLOW_LINKS)) {
            CoreConfig defaults = CoreConfig.defaults();
            save(defaults);
            return new ConfigLoadResult(
                    defaults,
                    ConfigLoadResult.Status.CREATED_DEFAULTS,
                    null);
        }
        if (Files.isSymbolicLink(configFile)) {
            throw new IOException("Configuration file must not be a symbolic link");
        }
        if (Files.size(configFile) > MAX_CONFIG_BYTES) {
            return recoverCorrupt();
        }

        String json = new String(Files.readAllBytes(configFile), StandardCharsets.UTF_8);
        try {
            int originalSchema = codec.readSchemaVersion(json);
            CoreConfig config = codec.decode(json);
            // Rewrite the accepted whitelist so unknown fields, including any
            // accidentally supplied secret, are never persisted by this
            // component - but only when the file is not already canonical.
            // load() is called on every GUI open, and an unconditional save()
            // fsyncs the config file on every inventory/chat/menu screen change.
            String canonical = codec.encode(config);
            if (!canonical.equals(json)) {
                AtomicFiles.writeUtf8(configFile, canonical);
            }
            ConfigLoadResult.Status status = originalSchema == CoreConfig.CURRENT_SCHEMA_VERSION
                    ? ConfigLoadResult.Status.LOADED
                    : ConfigLoadResult.Status.MIGRATED;
            return new ConfigLoadResult(config, status, null);
        } catch (RuntimeException invalidConfig) {
            return recoverCorrupt();
        }
    }

    public synchronized void save(CoreConfig config) throws IOException {
        if (config == null) {
            throw new IllegalArgumentException("Config is required");
        }
        AtomicFiles.writeUtf8(configFile, codec.encode(config));
    }

    public Path getConfigFile() {
        return configFile;
    }

    private ConfigLoadResult recoverCorrupt() throws IOException {
        Path quarantined = AtomicFiles.quarantine(configFile, clock);
        CoreConfig defaults = CoreConfig.defaults();
        save(defaults);
        return new ConfigLoadResult(
                defaults,
                ConfigLoadResult.Status.RECOVERED_CORRUPT,
                quarantined);
    }
}
