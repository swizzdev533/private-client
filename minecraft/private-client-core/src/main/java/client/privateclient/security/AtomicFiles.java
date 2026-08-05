package client.privateclient.security;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.channels.FileChannel;
import java.nio.charset.StandardCharsets;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.nio.file.StandardOpenOption;
import java.time.Clock;
import java.util.UUID;

public final class AtomicFiles {
    private AtomicFiles() {
    }

    public static void writeUtf8(Path target, String content) throws IOException {
        if (target == null || content == null) {
            throw new IllegalArgumentException("Target and content are required");
        }
        Path normalizedTarget = target.toAbsolutePath().normalize();
        Path parent = normalizedTarget.getParent();
        if (parent == null) {
            throw new IOException("Target must have a parent directory");
        }

        SafePaths.ensureSafeDirectory(parent);
        if (Files.exists(normalizedTarget) && Files.isSymbolicLink(normalizedTarget)) {
            throw new IOException("Refusing to replace a symbolic link");
        }

        Path temporary = Files.createTempFile(parent, "." + normalizedTarget.getFileName() + ".", ".tmp");
        boolean moved = false;
        try {
            byte[] bytes = content.getBytes(StandardCharsets.UTF_8);
            try (FileChannel channel = FileChannel.open(
                    temporary,
                    StandardOpenOption.WRITE,
                    StandardOpenOption.TRUNCATE_EXISTING)) {
                ByteBuffer buffer = ByteBuffer.wrap(bytes);
                while (buffer.hasRemaining()) {
                    channel.write(buffer);
                }
                channel.force(true);
            }

            try {
                Files.move(
                        temporary,
                        normalizedTarget,
                        StandardCopyOption.ATOMIC_MOVE,
                        StandardCopyOption.REPLACE_EXISTING);
            } catch (AtomicMoveNotSupportedException exception) {
                Files.move(temporary, normalizedTarget, StandardCopyOption.REPLACE_EXISTING);
            }
            moved = true;
        } finally {
            if (!moved) {
                Files.deleteIfExists(temporary);
            }
        }
    }

    public static Path quarantine(Path source, Clock clock) throws IOException {
        if (source == null || clock == null) {
            throw new IllegalArgumentException("Source and clock are required");
        }
        Path normalized = source.toAbsolutePath().normalize();
        if (!Files.exists(normalized)) {
            return normalized;
        }
        if (Files.isSymbolicLink(normalized)) {
            throw new IOException("Refusing to quarantine a symbolic link");
        }
        String backupName = normalized.getFileName()
                + ".corrupt-"
                + clock.millis()
                + "-"
                + UUID.randomUUID().toString()
                + ".bak";
        Path backup = normalized.resolveSibling(backupName);
        try {
            Files.move(normalized, backup, StandardCopyOption.ATOMIC_MOVE);
        } catch (AtomicMoveNotSupportedException exception) {
            Files.move(normalized, backup);
        }
        return backup;
    }
}
