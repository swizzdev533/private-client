package client.privateclient.security;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.fail;

import java.nio.file.Path;
import java.nio.file.Paths;
import org.junit.Test;

public final class SafePathsTest {
    @Test
    public void normalizesSafeRelativePath() {
        assertEquals(
                "cache/profiles/id/skin.png",
                SafePaths.normalizeRelative("cache/profiles/id/./skin.png"));
    }

    @Test
    public void rejectsParentTraversalAndAbsolutePath() {
        assertRejected("../outside");
        assertRejected(Paths.get("C:\\", "outside").toString());
    }

    private static void assertRejected(String value) {
        try {
            SafePaths.normalizeRelative(value);
            fail("Expected path rejection: " + value);
        } catch (IllegalArgumentException expected) {
            // Expected.
        }
    }
}
