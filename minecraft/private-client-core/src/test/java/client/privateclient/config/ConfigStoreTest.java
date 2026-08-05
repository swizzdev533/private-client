package client.privateclient.config;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertEquals;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Clock;
import java.time.Instant;
import java.time.ZoneOffset;
import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;

public final class ConfigStoreTest {
    @Rule
    public final TemporaryFolder temporaryFolder = new TemporaryFolder();

    @Test
    public void migratesVersionOneAndPersistsOnlyWhitelistedFields() throws Exception {
        Path configFile = temporaryFolder.newFolder("config").toPath().resolve("config.json");
        String versionOne = "{"
                + "\"schemaVersion\":1,"
                + "\"enabledModules\":[\"sprinttoggle\",\"unknown-module\"],"
                + "\"accessToken\":\"must-not-survive\","
                + "\"profileBridge\":true,"
                + "\"sessionGuard\":true"
                + "}";
        Files.write(configFile, versionOne.getBytes(StandardCharsets.UTF_8));
        ConfigStore store = new ConfigStore(configFile);

        ConfigLoadResult result = store.load();

        assertEquals(ConfigLoadResult.Status.MIGRATED, result.getStatus());
        assertTrue(result.getConfig().getModules().get("sprinttoggle"));
        assertFalse(result.getConfig().getModules().get("nametags"));
        assertFalse(result.getConfig().getModules().containsKey("unknown-module"));
        String persisted = new String(Files.readAllBytes(configFile), StandardCharsets.UTF_8);
        assertFalse(persisted.contains("accessToken"));
        assertFalse(persisted.contains("must-not-survive"));
        assertTrue(persisted.contains("\"schemaVersion\": 3"));
        assertTrue(result.getConfig().isDiscordPresenceEnabled());
    }

    @Test
    public void quarantinesCorruptConfigAndRecoversDefaults() throws Exception {
        Path configFile = temporaryFolder.newFolder("corrupt").toPath().resolve("config.json");
        Files.write(configFile, "{broken".getBytes(StandardCharsets.UTF_8));
        Clock fixed = Clock.fixed(Instant.parse("2026-01-02T03:04:05Z"), ZoneOffset.UTC);
        ConfigStore store = new ConfigStore(configFile, new ConfigCodec(), fixed);

        ConfigLoadResult result = store.load();

        assertEquals(ConfigLoadResult.Status.RECOVERED_CORRUPT, result.getStatus());
        assertTrue(result.getQuarantinedFile().isPresent());
        assertTrue(Files.exists(result.getQuarantinedFile().get()));
        assertTrue(Files.exists(configFile));
        assertTrue(result.getConfig().getModules().get("sprinttoggle"));
    }
}
