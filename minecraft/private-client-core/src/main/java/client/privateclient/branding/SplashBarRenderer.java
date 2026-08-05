package client.privateclient.branding;

import java.util.Iterator;
import net.minecraftforge.fml.common.ProgressManager;
import org.lwjgl.opengl.Display;
import org.lwjgl.opengl.GL11;

/**
 * Draws the Private Client loading indicator: one white bar instead of the three stacked Forge
 * progress bars with their raw mod titles and step counters.
 *
 * <p>Called from {@code SplashProgress$3.drawBar} after {@link SplashProgressTransformer} replaces
 * that method body. It runs on the splash thread while the game is still loading, so it must never
 * throw and must leave the fixed-function state the splash renderer expects.
 */
public final class SplashBarRenderer {
    private static final int MAXIMUM_BAR_WIDTH = 420;
    private static final float BAR_WIDTH_RATIO = 0.55F;
    private static final float BAR_HEIGHT = 3.0F;
    private static final float BAR_BOTTOM_MARGIN = 74.0F;
    private static final float TRACK_ALPHA = 0.16F;
    private static final float FILL_ALPHA = 0.95F;
    private static final float GLOW_WIDTH = 28.0F;
    /** Frames are ~100 FPS on the splash thread, so a small step keeps the fill smooth. */
    private static final float SMOOTHING = 0.10F;
    private static final int MAXIMUM_NESTED_BARS = 8;

    private static float displayedProgress;

    private SplashBarRenderer() {
    }

    /**
     * @param bar the Forge bar the splash asked to draw; only used as a liveness hint, the drawn
     *     value is the combined progress of every active bar.
     */
    public static void drawBar(ProgressManager.ProgressBar bar) {
        try {
            render(smooth(combinedProgress()));
        } catch (Throwable ignored) {
            // The splash must never take the game down; a missing bar is preferable to a crash.
        }
    }

    /** Nested Forge bars are a fraction of their parent's current step. */
    static float combinedProgress() {
        float[] steps = new float[MAXIMUM_NESTED_BARS];
        float[] totals = new float[MAXIMUM_NESTED_BARS];
        int depth = 0;
        Iterator<ProgressManager.ProgressBar> bars = ProgressManager.barIterator();
        while (bars.hasNext() && depth < MAXIMUM_NESTED_BARS) {
            ProgressManager.ProgressBar current = bars.next();
            if (current == null) {
                continue;
            }
            steps[depth] = current.getStep();
            totals[depth] = current.getSteps();
            depth++;
        }
        float progress = 0.0F;
        for (int index = depth - 1; index >= 0; index--) {
            float total = totals[index] <= 0.0F ? 1.0F : totals[index];
            progress = (steps[index] + progress) / total;
        }
        return clamp(progress);
    }

    private static float smooth(float target) {
        displayedProgress += (target - displayedProgress) * SMOOTHING;
        return clamp(displayedProgress);
    }

    private static float clamp(float value) {
        if (value < 0.0F || Float.isNaN(value)) {
            return 0.0F;
        }
        return value > 1.0F ? 1.0F : value;
    }

    private static void render(float progress) {
        int width = Display.getWidth();
        int height = Display.getHeight();
        // The splash ortho keeps a 640x480 centre, so 320/240 is the middle of the window.
        float centerX = 320.0F;
        float bottom = 240.0F + height / 2.0F;
        float top = bottom - BAR_BOTTOM_MARGIN;
        float halfWidth = Math.min(MAXIMUM_BAR_WIDTH, width * BAR_WIDTH_RATIO) / 2.0F;
        float left = centerX - halfWidth;
        float right = centerX + halfWidth;
        float fillRight = left + (right - left) * progress;

        GL11.glPushMatrix();
        try {
            GL11.glLoadIdentity();
            GL11.glDisable(GL11.GL_TEXTURE_2D);
            GL11.glEnable(GL11.GL_BLEND);
            GL11.glBlendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);

            GL11.glBegin(GL11.GL_QUADS);
            quad(left, top, right, top + BAR_HEIGHT, TRACK_ALPHA);
            if (fillRight > left) {
                quad(left, top, fillRight, top + BAR_HEIGHT, FILL_ALPHA);
                glow(fillRight, Math.min(fillRight + GLOW_WIDTH, right), top, top + BAR_HEIGHT);
            }
            GL11.glEnd();
        } finally {
            GL11.glColor4f(1.0F, 1.0F, 1.0F, 1.0F);
            GL11.glPopMatrix();
        }
    }

    private static void quad(float left, float top, float right, float bottom, float alpha) {
        GL11.glColor4f(1.0F, 1.0F, 1.0F, alpha);
        GL11.glVertex2f(left, top);
        GL11.glVertex2f(left, bottom);
        GL11.glVertex2f(right, bottom);
        GL11.glVertex2f(right, top);
    }

    /** A short fade past the fill edge so the bar reads as moving light, not a hard block. */
    private static void glow(float edge, float glowEnd, float top, float bottom) {
        if (glowEnd <= edge) {
            return;
        }
        GL11.glColor4f(1.0F, 1.0F, 1.0F, FILL_ALPHA);
        GL11.glVertex2f(edge, top);
        GL11.glVertex2f(edge, bottom);
        GL11.glColor4f(1.0F, 1.0F, 1.0F, 0.0F);
        GL11.glVertex2f(glowEnd, bottom);
        GL11.glVertex2f(glowEnd, top);
    }
}
