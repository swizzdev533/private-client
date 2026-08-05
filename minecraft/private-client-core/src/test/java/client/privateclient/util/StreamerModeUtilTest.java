package client.privateclient.util;

import static org.junit.Assert.assertEquals;

import java.util.Arrays;
import org.junit.Test;

public class StreamerModeUtilTest {

    @Test
    public void testSanitizeNameWhenEnabled() {
        assertEquals("???", StreamerModeUtil.sanitizeName("Notch", true));
        assertEquals("Notch", StreamerModeUtil.sanitizeName("Notch", false));
    }

    @Test
    public void testSanitizeTextWhenEnabled() {
        assertEquals("<???> Hello world", StreamerModeUtil.sanitizeText("<Player123> Hello world", true));
        assertEquals("<Player123> Hello world", StreamerModeUtil.sanitizeText("<Player123> Hello world", false));
    }

    @Test
    public void createsStablePlayerAliasText() {
        assertEquals("Player 1", StreamerModeUtil.playerAlias(1));
        assertEquals("Player 24", StreamerModeUtil.playerAlias(24));
    }

    @Test
    public void sanitizesKnownNamesAnywhereInServerMessages() {
        assertEquals(
                "??? dołącza do gry (3/8)!",
                StreamerModeUtil.sanitizeKnownNames(
                        "KocieRuchy533 dołącza do gry (3/8)!",
                        true,
                        Arrays.asList("KocieRuchy533")));
        assertEquals(
                "Gracz ??? pokonał ???!",
                StreamerModeUtil.sanitizeKnownNames(
                        "Gracz Alice_12 pokonał bob123!",
                        true,
                        Arrays.asList("Alice_12", "Bob123")));
    }

    @Test
    public void sanitizesJoiningPlayersBeforeTheyReachThePlayerList() {
        assertEquals(
                "MVP ??? dołącza do gry (4/8)!",
                StreamerModeUtil.sanitizeKnownNames(
                        "MVP p3rsuazja_ dołącza do gry (4/8)!",
                        true,
                        Arrays.<String>asList()));
        assertEquals(
                "??? dołącza do gry (5/8)!",
                StreamerModeUtil.sanitizeKnownNames(
                        "briczekk dołącza do gry (5/8)!",
                        true,
                        Arrays.<String>asList()));
        assertEquals(
                "??? joined the game",
                StreamerModeUtil.sanitizeKnownNames(
                        "Player123 joined the game",
                        true,
                        Arrays.<String>asList()));
    }
}
