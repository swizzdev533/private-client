package client.privateclient.srv;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import org.junit.Test;

import static org.junit.Assert.assertEquals;

public final class SrvResolverTest {
    @Test
    public void resolvesSrvTargetAndPortForDomainWithoutPort() {
        ResolvedServerAddress result = SrvResolver.resolve("privatemc.pl", answers("0 5 20147 privatemc.pl."));
        assertEquals("privatemc.pl", result.getHost());
        assertEquals(20147, result.getPort());
    }

    @Test
    public void explicitPortNeverPerformsSrvLookup() {
        SrvResolver.Lookup failingLookup = new SrvResolver.Lookup() {
            @Override
            public List<String> lookup(String name) {
                throw new AssertionError("SRV lookup must not run for an explicit port");
            }
        };
        ResolvedServerAddress result = SrvResolver.resolve("example.org:20147", failingLookup);
        assertEquals("example.org", result.getHost());
        assertEquals(20147, result.getPort());
    }

    @Test
    public void explicitDefaultPortAlsoSkipsSrvLookup() {
        ResolvedServerAddress result = SrvResolver.resolve("example.org:25565", answers("0 1 20000 other.org."));
        assertEquals("example.org", result.getHost());
        assertEquals(25565, result.getPort());
    }

    @Test
    public void missingOrBrokenSrvFallsBackToDirectConnection() {
        ResolvedServerAddress missing = SrvResolver.resolve("example.org", answers());
        ResolvedServerAddress broken = SrvResolver.resolve("example.org", answers("not a valid record"));
        assertEquals("example.org", missing.getHost());
        assertEquals(25565, missing.getPort());
        assertEquals("example.org", broken.getHost());
        assertEquals(25565, broken.getPort());
    }

    @Test
    public void lowerPriorityRecordWins() {
        ResolvedServerAddress result = SrvResolver.resolve("example.org",
                answers("10 100 25570 backup.example.org.", "0 0 25571 primary.example.org."));
        assertEquals("primary.example.org", result.getHost());
        assertEquals(25571, result.getPort());
    }

    private static SrvResolver.Lookup answers(final String... values) {
        return new SrvResolver.Lookup() {
            @Override
            public List<String> lookup(String name) {
                return values.length == 0 ? Collections.<String>emptyList() : Arrays.asList(values);
            }
        };
    }
}
