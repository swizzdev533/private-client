package client.privateclient.util;

import static org.junit.Assert.assertEquals;

import org.junit.Test;

public final class FpsDisplayUtilTest {
    @Test
    public void formatsFrameRate() {
        assertEquals("144 FPS", FpsDisplayUtil.format(144));
    }

    @Test
    public void clampsInvalidNegativeFrameRate() {
        assertEquals("0 FPS", FpsDisplayUtil.format(-1));
    }
}
