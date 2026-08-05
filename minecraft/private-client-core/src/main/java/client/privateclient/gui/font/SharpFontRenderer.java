package client.privateclient.gui.font;

import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.util.ResourceLocation;
import org.lwjgl.opengl.GL11;

/**
 * Antialiased text for Private Client's own screens.
 *
 * <p>Minecraft's {@code FontRenderer} draws from an 8x8 bitmap atlas with
 * nearest-neighbour filtering. That is correct for the game, but any Private
 * Client surface that scales text up (the menu clock ran at 3x) turns every
 * font pixel into a hard 3x3 block. This samples a {@link GlyphAtlas} linearly
 * instead, from a rasterisation well above display size.
 *
 * <p>Not a replacement for {@code FontRenderer}. Vanilla screens, chat, and
 * anything using section-sign formatting codes stay on the bitmap font.
 */
public final class SharpFontRenderer {
    private final GlyphAtlas atlas;
    private final ResourceLocation texture;

    private SharpFontRenderer(GlyphAtlas atlas, ResourceLocation texture) {
        this.atlas = atlas;
        this.texture = texture;
    }

    /**
     * Builds the atlas and registers it with the texture manager.
     *
     * @return the renderer, or {@code null} when font rasterisation is
     *     unavailable, in which case callers fall back to {@code FontRenderer}
     *     rather than failing the screen.
     */
    public static SharpFontRenderer create(Minecraft minecraft, String id) {
        if (minecraft == null || minecraft.getTextureManager() == null) {
            return null;
        }
        try {
            GlyphAtlas atlas = GlyphAtlas.build();
            ResourceLocation location =
                    minecraft
                            .getTextureManager()
                            .getDynamicTextureLocation(
                                    "privateclient_font_" + id,
                                    new DynamicTexture(atlas.getImage()));
            return new SharpFontRenderer(atlas, location);
        } catch (Throwable failure) {
            // A missing font, a headless quirk, or an AWT failure must never
            // take a screen down with it.
            return null;
        }
    }

    /** Width of {@code text} when drawn at {@code size} pixels. */
    public float getStringWidth(String text, float size) {
        if (text == null) {
            return 0.0F;
        }
        float total = 0.0F;
        for (int i = 0; i < text.length(); i++) {
            total += atlas.advanceOf(text.charAt(i));
        }
        return total * (size / GlyphAtlas.GLYPH_PX);
    }

    public void drawCentered(String text, float centerX, float y, float size, int color) {
        draw(text, centerX - getStringWidth(text, size) / 2.0F, y, size, color);
    }

    /**
     * Draws {@code text} with its ascent box starting at {@code (x, y)}.
     *
     * @param size glyph box height in GUI pixels
     * @param color ARGB, matching the convention used by {@code FontRenderer}
     */
    public void draw(String text, float x, float y, float size, int color) {
        if (text == null || text.isEmpty()) {
            return;
        }
        Minecraft minecraft = Minecraft.getMinecraft();
        if (minecraft == null || minecraft.getTextureManager() == null) {
            return;
        }
        minecraft.getTextureManager().bindTexture(texture);

        GlStateManager.enableTexture2D();
        GlStateManager.enableBlend();
        GlStateManager.tryBlendFuncSeparate(770, 771, 1, 0);
        GlStateManager.enableAlpha();
        // Hard alpha cutting would strip the antialiased edges this class
        // exists to produce.
        GlStateManager.alphaFunc(GL11.GL_GREATER, 0.01F);

        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MIN_FILTER, GL11.GL_LINEAR);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MAG_FILTER, GL11.GL_LINEAR);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_WRAP_S, GL11.GL_CLAMP);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_WRAP_T, GL11.GL_CLAMP);

        GlStateManager.color(
                ((color >> 16) & 0xFF) / 255.0F,
                ((color >> 8) & 0xFF) / 255.0F,
                (color & 0xFF) / 255.0F,
                ((color >>> 24) & 0xFF) / 255.0F);

        float scale = size / GlyphAtlas.GLYPH_PX;
        float cellWidth = 1.0F / GlyphAtlas.COLUMNS;
        float cellHeight = 1.0F / atlas.getRows();
        float drawn = atlas.getCell() * scale;
        // The glyph sits inside a padded cell; shifting the quad back by the
        // pad keeps the visual origin where the caller asked for it.
        float offset = GlyphAtlas.PAD * scale;

        Tessellator tessellator = Tessellator.getInstance();
        WorldRenderer worldrenderer = tessellator.getWorldRenderer();
        worldrenderer.begin(7, DefaultVertexFormats.POSITION_TEX);

        float cursor = x;
        for (int i = 0; i < text.length(); i++) {
            char character = text.charAt(i);
            int cell = atlas.cellOf(character);
            if (cell < 0) {
                cursor += atlas.advanceOf(character) * scale;
                continue;
            }
            int column = cell % GlyphAtlas.COLUMNS;
            int row = cell / GlyphAtlas.COLUMNS;

            float u0 = column * cellWidth;
            float v0 = row * cellHeight;
            float left = cursor - offset;
            float top = y - offset;
            float right = left + drawn;
            float bottom = top + drawn;

            worldrenderer.pos(left, bottom, 0.0D).tex(u0, v0 + cellHeight).endVertex();
            worldrenderer.pos(right, bottom, 0.0D).tex(u0 + cellWidth, v0 + cellHeight).endVertex();
            worldrenderer.pos(right, top, 0.0D).tex(u0 + cellWidth, v0).endVertex();
            worldrenderer.pos(left, top, 0.0D).tex(u0, v0).endVertex();

            cursor += atlas.advanceOf(character) * scale;
        }
        tessellator.draw();

        GlStateManager.alphaFunc(GL11.GL_GREATER, 0.1F);
        GlStateManager.color(1.0F, 1.0F, 1.0F, 1.0F);
    }
}
