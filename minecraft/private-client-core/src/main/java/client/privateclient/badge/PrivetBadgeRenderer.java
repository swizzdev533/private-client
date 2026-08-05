package client.privateclient.badge;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.FontRenderer;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.util.ResourceLocation;
import org.lwjgl.opengl.GL11;

/**
 * Draws the Private Client association badge to the left of a nickname
 * (Lunar-style placement).
 *
 * <p>Asset is a 64×64 white-on-transparent mark with antialiased edges, drawn
 * with linear filtering so the outline stays crisp instead of showing nearest-
 * neighbour stair-stepping. The source art uses thick strokes; the thin-line
 * variant of the same logo turns to mush below ~24px and must not be used here.
 */
public final class PrivetBadgeRenderer {
    public static final ResourceLocation TEXTURE =
            new ResourceLocation("privateclientcore", "textures/gui/privet_badge.png");
    /** Source texture is 64×64, giving 4x headroom over the nametag draw size. */
    public static final int TEXTURE_SIZE = 64;
    /**
     * Nametag draw size in glyph units. The font glyph band is 8 units tall, so
     * this sits just above cap height — enough presence to read as a mark, small
     * enough to stay inside the nametag plate. Larger values push the badge past
     * the plate, where the camera-ward Z offset makes it clip into world geometry.
     */
    public static final int BADGE_SIZE = 10;
    /** TAB rows are 8px tall — keep the mark inside the slot. */
    public static final int TAB_BADGE_SIZE = 8;
    public static final int BADGE_GAP = 2;
    /** Pull badge slightly toward the camera so it does not z-fight the plate. */
    private static final float NAMETAG_Z = -0.01F;
    /**
     * Discard fully transparent fragments only. A 0.5 cut would eat the
     * antialiased stroke edges and break the outline into dashes.
     */
    private static final float ALPHA_CUTOFF = 0.01F;

    private PrivetBadgeRenderer() {
    }

    public static int badgeAdvance() {
        return BADGE_SIZE + BADGE_GAP;
    }

    public static int tabBadgeAdvance() {
        return TAB_BADGE_SIZE + BADGE_GAP;
    }

    /** Top Y of the badge when vertically centered on the nametag glyph band (0..8). */
    public static double nametagBadgeY() {
        return (8.0D - BADGE_SIZE) / 2.0D;
    }

    public static int backgroundTop(boolean showBadge) {
        if (!showBadge) {
            return -1;
        }
        return Math.min(-1, (int) Math.floor(nametagBadgeY()));
    }

    public static int backgroundBottom(boolean showBadge) {
        if (!showBadge) {
            return 8;
        }
        return Math.max(8, (int) Math.ceil(nametagBadgeY() + BADGE_SIZE));
    }

    /**
     * @return X coordinate where the name string should start (centered group)
     */
    public static int resolveTextX(FontRenderer font, String name, boolean showBadge) {
        int nameWidth = font.getStringWidth(name);
        int totalWidth = nameWidth + (showBadge ? badgeAdvance() : 0);
        int left = -totalWidth / 2;
        if (showBadge) {
            return left + badgeAdvance();
        }
        return left;
    }

    public static void drawNametagBadge(FontRenderer font, String name, boolean showBadge) {
        if (!showBadge || font == null) {
            return;
        }
        int nameWidth = font.getStringWidth(name);
        int totalWidth = nameWidth + badgeAdvance();
        int left = -totalWidth / 2;
        drawBadge(left, nametagBadgeY(), NAMETAG_Z, 1.0F, BADGE_SIZE);
    }

    public static void drawBadge(double x, double y) {
        drawBadge(x, y, 0.0F, 1.0F, BADGE_SIZE);
    }

    public static void drawScreenBadge(int x, int y) {
        drawBadge(x, y, 0.0F, 1.0F, TAB_BADGE_SIZE);
    }

    public static void drawBadge(double x, double y, float z, float alpha, int size) {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc == null || mc.getTextureManager() == null || alpha <= 0.0F || size <= 0) {
            return;
        }

        mc.getTextureManager().bindTexture(TEXTURE);

        // GlStateManager caches GL_FOG, so disableFog() no-ops once the cache has
        // desynced from real GL state and the mark picks up water/lava fog colour.
        // Drive GL directly and restore whatever was actually set.
        boolean fogWasEnabled = GL11.glIsEnabled(GL11.GL_FOG);

        GlStateManager.enableTexture2D();
        GlStateManager.disableLighting();
        GL11.glDisable(GL11.GL_FOG);
        // Antialiased mark: blend the stroke edges instead of hard-cutting them.
        // The texture is white-on-transparent, so blending cannot pull in dark
        // fringe the way a colour-keyed asset would.
        GlStateManager.enableBlend();
        GlStateManager.tryBlendFuncSeparate(770, 771, 1, 0);
        GlStateManager.enableAlpha();
        GlStateManager.alphaFunc(GL11.GL_GREATER, ALPHA_CUTOFF);
        GlStateManager.color(1.0F, 1.0F, 1.0F, alpha);

        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MIN_FILTER, GL11.GL_LINEAR);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MAG_FILTER, GL11.GL_LINEAR);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_WRAP_S, GL11.GL_CLAMP);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_WRAP_T, GL11.GL_CLAMP);

        Tessellator tessellator = Tessellator.getInstance();
        WorldRenderer worldrenderer = tessellator.getWorldRenderer();
        worldrenderer.begin(7, DefaultVertexFormats.POSITION_TEX_COLOR);
        int a = Math.max(0, Math.min(255, (int) (alpha * 255.0F)));
        worldrenderer.pos(x, y + size, z).tex(0.0D, 1.0D).color(255, 255, 255, a).endVertex();
        worldrenderer.pos(x + size, y + size, z).tex(1.0D, 1.0D).color(255, 255, 255, a).endVertex();
        worldrenderer.pos(x + size, y, z).tex(1.0D, 0.0D).color(255, 255, 255, a).endVertex();
        worldrenderer.pos(x, y, z).tex(0.0D, 0.0D).color(255, 255, 255, a).endVertex();
        tessellator.draw();

        GlStateManager.enableBlend();
        GlStateManager.tryBlendFuncSeparate(770, 771, 1, 0);
        GlStateManager.alphaFunc(GL11.GL_GREATER, 0.1F);
        GlStateManager.color(1.0F, 1.0F, 1.0F, 1.0F);
        GlStateManager.enableTexture2D();
        if (fogWasEnabled) {
            GL11.glEnable(GL11.GL_FOG);
        }
    }

    public static int backgroundHalfWidth(FontRenderer font, String name, boolean showBadge) {
        int nameWidth = font.getStringWidth(name);
        int total = nameWidth + (showBadge ? badgeAdvance() : 0);
        return (total + 1) / 2;
    }
}
