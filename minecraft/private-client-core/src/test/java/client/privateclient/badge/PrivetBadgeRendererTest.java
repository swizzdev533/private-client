package client.privateclient.badge;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class PrivetBadgeRendererTest {
    @Test
    public void badgeAdvanceMatchesSizePlusGap() {
        assertEquals(
                PrivetBadgeRenderer.BADGE_SIZE + PrivetBadgeRenderer.BADGE_GAP,
                PrivetBadgeRenderer.badgeAdvance());
    }

    /**
     * A badge taller than the plate pokes past it, and the camera-ward Z offset
     * then clips it into world geometry. Keep it bounded by the glyph band.
     */
    @Test
    public void nametagBadgeStaysInsideThePlate() {
        int top = PrivetBadgeRenderer.backgroundTop(true);
        int bottom = PrivetBadgeRenderer.backgroundBottom(true);
        double badgeTop = PrivetBadgeRenderer.nametagBadgeY();
        double badgeBottom = badgeTop + PrivetBadgeRenderer.BADGE_SIZE;

        assertTrue("badge overflows the plate top", badgeTop >= top);
        assertTrue("badge overflows the plate bottom", badgeBottom <= bottom);
        assertTrue("badge should not tower over the 8u glyph band",
                PrivetBadgeRenderer.BADGE_SIZE <= 12);
    }

    @Test
    public void plateStaysCloseToVanillaHeight() {
        assertEquals(-1, PrivetBadgeRenderer.backgroundTop(false));
        assertEquals(8, PrivetBadgeRenderer.backgroundBottom(false));
        assertTrue(PrivetBadgeRenderer.backgroundBottom(true)
                - PrivetBadgeRenderer.backgroundTop(true) <= 12);
    }

    @Test
    public void textureHasHeadroomOverEveryDrawSize() {
        assertTrue(PrivetBadgeRenderer.TEXTURE_SIZE >= PrivetBadgeRenderer.BADGE_SIZE * 2);
        assertTrue(PrivetBadgeRenderer.TEXTURE_SIZE >= PrivetBadgeRenderer.TAB_BADGE_SIZE * 2);
    }
}
