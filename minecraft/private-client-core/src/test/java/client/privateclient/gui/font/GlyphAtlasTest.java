package client.privateclient.gui.font;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.awt.image.BufferedImage;
import org.junit.Test;

public class GlyphAtlasTest {
    private static int coverageIn(BufferedImage image, int x0, int y0, int x1, int y1) {
        int covered = 0;
        for (int y = y0; y < y1; y++) {
            for (int x = x0; x < x1; x++) {
                if (((image.getRGB(x, y) >>> 24) & 0xFF) > 8) {
                    covered++;
                }
            }
        }
        return covered;
    }

    private static int[] cellOrigin(GlyphAtlas atlas, int cell) {
        return new int[] {
            (cell % GlyphAtlas.COLUMNS) * atlas.getCell(),
            (cell / GlyphAtlas.COLUMNS) * atlas.getCell(),
        };
    }

    @Test
    public void resolvesAnInstalledFontRatherThanTheDialogFallback() {
        GlyphAtlas atlas = GlyphAtlas.build();
        assertTrue(
                "resolved family should be a real installed font, got " + atlas.getFamily(),
                atlas.getFamily() != null && !atlas.getFamily().isEmpty());
    }

    @Test
    public void atlasIsLargeEnoughForEveryGlyph() {
        GlyphAtlas atlas = GlyphAtlas.build();
        BufferedImage image = atlas.getImage();
        assertEquals(GlyphAtlas.COLUMNS * atlas.getCell(), image.getWidth());
        assertTrue(atlas.getRows() * GlyphAtlas.COLUMNS >= GlyphAtlas.CHARSET.length());
        assertEquals(atlas.getRows() * atlas.getCell(), image.getHeight());
    }

    @Test
    public void rastersEveryVisibleCharacterIncludingPolishDiacritics() {
        GlyphAtlas atlas = GlyphAtlas.build();
        BufferedImage image = atlas.getImage();

        for (char character : "AZaz09Ćłńśźż?@".toCharArray()) {
            int cell = atlas.cellOf(character);
            assertTrue("missing charset entry for '" + character + "'", cell >= 0);
            int[] origin = cellOrigin(atlas, cell);
            int covered =
                    coverageIn(
                            image,
                            origin[0],
                            origin[1],
                            origin[0] + atlas.getCell(),
                            origin[1] + atlas.getCell());
            assertTrue("'" + character + "' rasterised empty", covered > 0);
        }
    }

    /**
     * Linear filtering samples neighbouring texels, so any ink reaching a cell
     * edge would smear into the adjacent glyph at draw time.
     */
    @Test
    public void glyphsStayInsideTheirPaddedCell() {
        GlyphAtlas atlas = GlyphAtlas.build();
        BufferedImage image = atlas.getImage();

        for (char character : "WM@gjy_|".toCharArray()) {
            int cell = atlas.cellOf(character);
            int[] origin = cellOrigin(atlas, cell);
            int x0 = origin[0];
            int y0 = origin[1];
            int x1 = x0 + atlas.getCell();
            int y1 = y0 + atlas.getCell();

            assertEquals("'" + character + "' bleeds off the left edge",
                    0, coverageIn(image, x0, y0, x0 + 2, y1));
            assertEquals("'" + character + "' bleeds off the right edge",
                    0, coverageIn(image, x1 - 2, y0, x1, y1));
            assertEquals("'" + character + "' bleeds off the top edge",
                    0, coverageIn(image, x0, y0, x1, y0 + 2));
            assertEquals("'" + character + "' bleeds off the bottom edge",
                    0, coverageIn(image, x0, y1 - 2, x1, y1));
        }
    }

    /** A proportional clock shifts sideways every time the minute rolls over. */
    @Test
    public void digitsAreTabular() {
        GlyphAtlas atlas = GlyphAtlas.build();
        float reference = atlas.advanceOf('0');
        assertTrue(reference > 0.0F);
        for (char digit = '0'; digit <= '9'; digit++) {
            assertEquals(
                    "digit '" + digit + "' is not tabular",
                    reference,
                    atlas.advanceOf(digit),
                    0.0001F);
        }
    }

    @Test
    public void unknownCharactersDoNotAdvanceTheCursor() {
        GlyphAtlas atlas = GlyphAtlas.build();
        assertEquals(-1, atlas.cellOf('中'));
        assertEquals(0.0F, atlas.advanceOf('中'), 0.0001F);
    }
}
