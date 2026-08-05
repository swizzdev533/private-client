package client.privateclient.security;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.InvalidPathException;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.nio.file.Paths;

public final class SafePaths {
    private SafePaths() {
    }

    public static Path normalizeRoot(Path root) {
        if (root == null) {
            throw new IllegalArgumentException("Root path is required");
        }
        return root.toAbsolutePath().normalize();
    }

    public static Path resolveRelative(Path root, String relativePath) {
        Path normalizedRoot = normalizeRoot(root);
        Path relative = parseRelative(relativePath);
        Path resolved = normalizedRoot.resolve(relative).normalize();
        if (!resolved.startsWith(normalizedRoot)) {
            throw new IllegalArgumentException("Path escapes the Private Client data directory");
        }
        return resolved;
    }

    public static String normalizeRelative(String relativePath) {
        Path normalized = parseRelative(relativePath).normalize();
        String value = normalized.toString().replace('\\', '/');
        if (".".equals(value)) {
            return "";
        }
        return value;
    }

    public static void ensureSafeDirectory(Path directory) throws IOException {
        Path normalized = normalizeRoot(directory);
        Files.createDirectories(normalized);

        Path current = normalized.getRoot();
        for (Path component : normalized) {
            current = current == null ? component : current.resolve(component);
            if (Files.exists(current, LinkOption.NOFOLLOW_LINKS) && Files.isSymbolicLink(current)) {
                throw new IOException("Symbolic links are not allowed in the Private Client data path");
            }
        }
        if (!Files.isDirectory(normalized, LinkOption.NOFOLLOW_LINKS)) {
            throw new IOException("Expected a directory: " + normalized);
        }
    }

    private static Path parseRelative(String value) {
        if (value == null) {
            throw new IllegalArgumentException("Relative path is required");
        }
        if (value.indexOf('\0') >= 0) {
            throw new IllegalArgumentException("Relative path contains a NUL byte");
        }

        final Path path;
        try {
            path = Paths.get(value);
        } catch (InvalidPathException exception) {
            throw new IllegalArgumentException("Relative path is invalid", exception);
        }
        if (path.isAbsolute()) {
            throw new IllegalArgumentException("Absolute paths are not allowed");
        }
        Path normalized = path.normalize();
        if (normalized.startsWith("..")) {
            throw new IllegalArgumentException("Parent traversal is not allowed");
        }
        return normalized;
    }
}
