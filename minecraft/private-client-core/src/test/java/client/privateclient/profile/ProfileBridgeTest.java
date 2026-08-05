package client.privateclient.profile;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Instant;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import java.util.UUID;
import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;

public final class ProfileBridgeTest {
    @Rule
    public final TemporaryFolder temporaryFolder = new TemporaryFolder();

    @Test
    public void atomicallyPublishesOnlyTheSixAllowedFields() throws Exception {
        Path dataRoot = temporaryFolder.newFolder("data").toPath();
        Path profileFile = dataRoot.resolve("profiles/profile.json");
        ProfileBridge bridge = new ProfileBridge(dataRoot, profileFile);
        UUID uuid = UUID.fromString("12345678-1234-1234-9234-1234567890ab");
        PlayerProfile profile = new PlayerProfile(
                1,
                "Example_Player",
                uuid,
                SkinModel.SLIM,
                "cache/profiles/" + uuid + "/skin.png",
                Instant.parse("2026-01-01T12:00:00Z"));

        bridge.publish(profile);

        assertEquals("Example_Player", bridge.read().get().getUsername());
        String json = new String(Files.readAllBytes(profileFile), StandardCharsets.UTF_8);
        JsonObject root = new JsonParser().parse(json).getAsJsonObject();
        Set<String> keys = new HashSet<String>();
        root.entrySet().forEach(entry -> keys.add(entry.getKey()));
        assertEquals(new HashSet<String>(Arrays.asList(
                "schemaVersion",
                "username",
                "uuid",
                "skinModel",
                "skinPath",
                "updatedAt")), keys);
        assertFalse(json.toLowerCase(java.util.Locale.ROOT).contains("token"));
    }

    @Test
    public void rejectsTraversalAndMismatchedSkinCache() {
        UUID uuid = UUID.fromString("12345678-1234-1234-9234-1234567890ab");
        try {
            new PlayerProfile(
                    1,
                    "Example",
                    uuid,
                    SkinModel.CLASSIC,
                    "../outside.png",
                    Instant.now());
            fail("Expected traversal rejection");
        } catch (IllegalArgumentException expected) {
            assertTrue(expected.getMessage().contains("traversal"));
        }
    }

    @Test
    public void corruptProfileIsQuarantinedInsteadOfBeingTrusted() throws Exception {
        Path dataRoot = temporaryFolder.newFolder("corrupt-data").toPath();
        Path profileFile = dataRoot.resolve("profiles/profile.json");
        Files.createDirectories(profileFile.getParent());
        Files.write(profileFile, "{\"accessToken\":\"secret\"}".getBytes(StandardCharsets.UTF_8));
        ProfileBridge bridge = new ProfileBridge(dataRoot, profileFile);

        assertFalse(bridge.read().isPresent());
        assertFalse(Files.exists(profileFile));
        assertTrue(Files.list(profileFile.getParent())
                .anyMatch(path -> path.getFileName().toString().contains(".corrupt-")));
    }
}
