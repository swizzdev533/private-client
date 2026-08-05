package client.privateclient.association.net;

import static org.junit.Assert.fail;

import java.util.Collections;
import org.junit.Test;

public class AssociationHttpClientTest {
    @Test
    public void rejectsNonAllowlistedHost() {
        AssociationHttpClient client = new AssociationHttpClient(
                Collections.singleton("private-client-association.vercel.app"));
        try {
            client.heartbeat("https://evil.example/api/v1/presence", "{\"schemaVersion\":1}");
            fail("expected IOException");
        } catch (Exception expected) {
            // ok
        }
    }

    @Test
    public void rejectsPlainHttp() {
        AssociationHttpClient client = new AssociationHttpClient(
                Collections.singleton("private-client-association.vercel.app"));
        try {
            client.heartbeat("http://private-client-association.vercel.app/api/v1/presence", "{}");
            fail("expected IOException");
        } catch (Exception expected) {
            // ok
        }
    }
}
