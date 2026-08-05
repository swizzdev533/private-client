package client.privateclient.profile;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.awt.image.BufferedImage;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.UUID;
import javax.imageio.ImageIO;
import org.junit.Test;

public final class LocalSkinCacheTest {
    @Test
    public void loadsOnlyTheExpectedMinecraftSkinDimensions() throws Exception {
        Path root = workspaceTemporary("private-client-skin-cache");
        UUID profileId = UUID.fromString("9236d2e2-5705-4807-8b1d-99ae84f64476");
        Path skin = root.resolve("cache/profiles").resolve(profileId.toString()).resolve("skin.png");
        Files.createDirectories(skin.getParent());

        ImageIO.write(new BufferedImage(64, 64, BufferedImage.TYPE_INT_ARGB), "png", skin.toFile());
        LocalSkinCache cache = new LocalSkinCache(root);
        assertTrue(cache.load(profileId).isPresent());

        ImageIO.write(new BufferedImage(64, 32, BufferedImage.TYPE_INT_ARGB), "png", skin.toFile());
        BufferedImage normalizedLegacy = cache.load(profileId).get();
        assertEquals(64, normalizedLegacy.getWidth());
        assertEquals(64, normalizedLegacy.getHeight());

        ImageIO.write(new BufferedImage(32, 32, BufferedImage.TYPE_INT_ARGB), "png", skin.toFile());
        assertFalse(cache.load(profileId).isPresent());
    }

    @Test
    public void rejectsAFalseOrOversizedPngHeaderBeforeDecoding() throws Exception {
        Path root = workspaceTemporary("private-client-hostile-skin-cache");
        UUID profileId = UUID.randomUUID();
        Path skin = root.resolve("cache/profiles").resolve(profileId.toString()).resolve("skin.png");
        Files.createDirectories(skin.getParent());

        Files.write(skin, new byte[32]);
        LocalSkinCache cache = new LocalSkinCache(root);
        assertFalse(cache.load(profileId).isPresent());

        byte[] oversizedHeader = {
            (byte) 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
            0x00, 0x01, (byte) 0x86, (byte) 0xA0,
            0x00, 0x01, (byte) 0x86, (byte) 0xA0
        };
        Files.write(skin, oversizedHeader);
        assertFalse(cache.load(profileId).isPresent());
    }

    @Test
    public void missingProfileSkinUsesTheCallerFallback() throws Exception {
        Path root = workspaceTemporary("private-client-empty-skin-cache");
        LocalSkinCache cache = new LocalSkinCache(root);
        assertFalse(cache.load(UUID.randomUUID()).isPresent());
    }

    private static Path workspaceTemporary(String prefix) throws Exception {
        Path parent = Paths.get("build", "test-skin-cache").toAbsolutePath();
        Files.createDirectories(parent);
        return Files.createTempDirectory(parent, prefix);
    }
}
