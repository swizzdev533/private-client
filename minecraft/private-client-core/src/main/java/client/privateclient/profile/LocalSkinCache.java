package client.privateclient.profile;

import client.privateclient.security.SafePaths;
import java.awt.Graphics2D;
import java.awt.image.BufferedImage;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.Optional;
import java.util.UUID;
import javax.imageio.ImageIO;

/** Reads only launcher-validated skin PNGs from the local profile cache. */
public final class LocalSkinCache {
    private static final long MAX_SKIN_BYTES = 8L * 1024L * 1024L;
    private static final byte[] PNG_SIGNATURE = {
        (byte) 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A
    };
    private final Path dataRoot;

    public LocalSkinCache(Path dataRoot) {
        this.dataRoot = SafePaths.normalizeRoot(dataRoot);
    }

    public Optional<BufferedImage> load(UUID profileId) throws IOException {
        if (profileId == null) {
            return Optional.empty();
        }
        Path skin = SafePaths.resolveRelative(
                dataRoot,
                "cache/profiles/" + profileId.toString() + "/skin.png");
        if (!Files.isRegularFile(skin, LinkOption.NOFOLLOW_LINKS)
                || Files.isSymbolicLink(skin)) {
            return Optional.empty();
        }
        long size = Files.size(skin);
        if (size < 24 || size > MAX_SKIN_BYTES) {
            return Optional.empty();
        }

        Path realRoot = dataRoot.toRealPath();
        Path realSkin = skin.toRealPath(LinkOption.NOFOLLOW_LINKS);
        if (!realSkin.startsWith(realRoot)) {
            return Optional.empty();
        }

        byte[] encoded = readBounded(realSkin);
        if (!hasExpectedPngHeader(encoded)) {
            return Optional.empty();
        }
        BufferedImage image;
        try {
            image = ImageIO.read(new ByteArrayInputStream(encoded));
        } catch (RuntimeException malformedImage) {
            return Optional.empty();
        }
        if (image == null || image.getWidth() != 64
                || (image.getHeight() != 32 && image.getHeight() != 64)) {
            return Optional.empty();
        }
        return Optional.of(normalizeLegacySkin(image));
    }

    private static byte[] readBounded(Path skin) throws IOException {
        try (InputStream input = Files.newInputStream(
                skin, StandardOpenOption.READ, LinkOption.NOFOLLOW_LINKS)) {
            ByteArrayOutputStream output = new ByteArrayOutputStream(8192);
            byte[] buffer = new byte[8192];
            long total = 0L;
            int count;
            while ((count = input.read(buffer)) >= 0) {
                total += count;
                if (total > MAX_SKIN_BYTES) {
                    throw new IOException("Skin cache entry exceeds the size limit");
                }
                output.write(buffer, 0, count);
            }
            return output.toByteArray();
        }
    }

    private static boolean hasExpectedPngHeader(byte[] encoded) {
        if (encoded.length < 24) {
            return false;
        }
        for (int index = 0; index < PNG_SIGNATURE.length; index++) {
            if (encoded[index] != PNG_SIGNATURE[index]) {
                return false;
            }
        }
        if (readInt(encoded, 8) != 13
                || encoded[12] != 'I' || encoded[13] != 'H'
                || encoded[14] != 'D' || encoded[15] != 'R') {
            return false;
        }
        int width = readInt(encoded, 16);
        int height = readInt(encoded, 20);
        return width == 64 && (height == 32 || height == 64);
    }

    private static int readInt(byte[] encoded, int offset) {
        return (encoded[offset] & 0xFF) << 24
                | (encoded[offset + 1] & 0xFF) << 16
                | (encoded[offset + 2] & 0xFF) << 8
                | encoded[offset + 3] & 0xFF;
    }

    private static BufferedImage normalizeLegacySkin(BufferedImage image) {
        if (image.getHeight() == 64) {
            return image;
        }
        BufferedImage normalized = new BufferedImage(64, 64, BufferedImage.TYPE_INT_ARGB);
        Graphics2D graphics = normalized.createGraphics();
        try {
            graphics.drawImage(image, 0, 0, null);
        } finally {
            graphics.dispose();
        }
        return normalized;
    }
}
