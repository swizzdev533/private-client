package client.privateclient.gui.font;

import java.awt.Color;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.geom.Rectangle2D;
import java.awt.image.BufferedImage;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;

/**
 * Rasterises an outline font into a padded glyph grid.
 *
 * <p>Deliberately free of any Minecraft or OpenGL dependency: this is the part
 * worth testing, and it can run in a plain JVM.
 *
 * <p>Private Client does not ship a font file. Bundling one would redistribute
 * third-party content, so the typeface is resolved from what is installed on
 * the machine, with a documented preference order and a guaranteed fallback.
 */
public final class GlyphAtlas {
    /** Rasterisation size. Display sizes run 8-34px, so this leaves headroom. */
    public static final int GLYPH_PX = 64;
    /**
     * Cell padding. Covers antialiasing spill past the glyph box, and the fact
     * that wide glyphs ('W', '@') advance close to the full rasterisation size.
     * Too little here and a glyph bleeds into its neighbour under linear
     * filtering.
     */
    public static final int PAD = 12;
    public static final int COLUMNS = 12;

    public static final String CHARSET =
            " !\"#$%&'()*+,-./0123456789:;<=>?@"
                    + "ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`"
                    + "abcdefghijklmnopqrstuvwxyz{|}~"
                    + "ĄĆĘŁŃÓŚŹŻąćęłńóśźż";

    /** Preferred families, best first, resolved against what is installed. */
    private static final String[] PREFERRED_FAMILIES = {
        "Segoe UI Semibold", "Segoe UI", "Inter", "Roboto", "Helvetica Neue", "Arial",
    };

    private final BufferedImage image;
    private final Map<Character, Integer> cells;
    private final float[] advances;
    private final int rows;
    private final int cell;
    private final float digitAdvance;
    private final String family;

    private GlyphAtlas(
            BufferedImage image,
            Map<Character, Integer> cells,
            float[] advances,
            int rows,
            int cell,
            float digitAdvance,
            String family) {
        this.image = image;
        this.cells = Collections.unmodifiableMap(cells);
        this.advances = advances;
        this.rows = rows;
        this.cell = cell;
        this.digitAdvance = digitAdvance;
        this.family = family;
    }

    private static Graphics2D prepare(BufferedImage target, Font font) {
        Graphics2D graphics = target.createGraphics();
        graphics.setRenderingHint(
                RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
        graphics.setRenderingHint(
                RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
        graphics.setRenderingHint(
                RenderingHints.KEY_FRACTIONALMETRICS, RenderingHints.VALUE_FRACTIONALMETRICS_ON);
        graphics.setRenderingHint(
                RenderingHints.KEY_STROKE_CONTROL, RenderingHints.VALUE_STROKE_PURE);
        graphics.setFont(font);
        graphics.setColor(Color.WHITE);
        return graphics;
    }

    public static GlyphAtlas build() {
        Font font = resolveFont();
        int rows = (CHARSET.length() + COLUMNS - 1) / COLUMNS;

        // Measure first. A font's ascent plus descent routinely exceeds its
        // point size, and the widest glyphs can out-advance it too, so sizing
        // the cell from GLYPH_PX lets descenders spill into the neighbouring
        // cell and smear there under linear filtering.
        BufferedImage scratch = new BufferedImage(1, 1, BufferedImage.TYPE_INT_ARGB);
        Graphics2D probe = prepare(scratch, font);
        FontMetrics metrics = probe.getFontMetrics();
        int ascent = metrics.getAscent();
        int descent = metrics.getDescent();

        float[] advances = new float[CHARSET.length()];
        float widest = 0.0F;
        float digitAdvance = 0.0F;
        for (int index = 0; index < CHARSET.length(); index++) {
            char character = CHARSET.charAt(index);
            Rectangle2D bounds = metrics.getStringBounds(String.valueOf(character), probe);
            advances[index] = (float) bounds.getWidth();
            widest = Math.max(widest, advances[index]);
            if (character >= '0' && character <= '9') {
                digitAdvance = Math.max(digitAdvance, advances[index]);
            }
        }
        probe.dispose();

        int cell = PAD * 2 + (int) Math.ceil(Math.max(ascent + descent, widest));

        BufferedImage image =
                new BufferedImage(COLUMNS * cell, rows * cell, BufferedImage.TYPE_INT_ARGB);
        Graphics2D graphics = prepare(image, font);

        Map<Character, Integer> cells = new HashMap<Character, Integer>();
        for (int index = 0; index < CHARSET.length(); index++) {
            char character = CHARSET.charAt(index);
            int column = index % COLUMNS;
            int row = index / COLUMNS;
            graphics.drawString(
                    String.valueOf(character), column * cell + PAD, row * cell + PAD + ascent);
            cells.put(Character.valueOf(character), Integer.valueOf(index));
        }
        graphics.dispose();

        return new GlyphAtlas(image, cells, advances, rows, cell, digitAdvance, font.getFamily());
    }

    private static Font resolveFont() {
        for (String family : PREFERRED_FAMILIES) {
            Font candidate = new Font(family, Font.PLAIN, GLYPH_PX);
            // Java silently substitutes "Dialog" for a family that is not
            // installed, so comparing the resolved family is how you detect a
            // real hit without enumerating every font on the system.
            if (!"Dialog".equalsIgnoreCase(candidate.getFamily())) {
                return candidate;
            }
        }
        return new Font(Font.SANS_SERIF, Font.PLAIN, GLYPH_PX);
    }

    public BufferedImage getImage() {
        return image;
    }

    public int getRows() {
        return rows;
    }

    /** Square cell size, derived from real font metrics rather than point size. */
    public int getCell() {
        return cell;
    }

    public String getFamily() {
        return family;
    }

    /** Cell index for a character, or {@code -1} when it is not in the charset. */
    public int cellOf(char character) {
        Integer cell = cells.get(Character.valueOf(character));
        return cell == null ? -1 : cell.intValue();
    }

    /** Advance at rasterisation scale. Digits are tabular. */
    public float advanceOf(char character) {
        if (character >= '0' && character <= '9') {
            // Proportional digits make a live clock twitch sideways every time
            // the minute rolls over.
            return digitAdvance;
        }
        int cell = cellOf(character);
        return cell < 0 ? 0.0F : advances[cell];
    }
}
