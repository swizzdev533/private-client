package client.privateclient.auth;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public final class SessionPolicyTest {
    private static final String UUID_VALUE = "12345678-1234-1234-9234-1234567890ab";

    @Test
    public void acceptsOnlyStructurallyAuthenticatedMojangSession() {
        SessionPolicy policy = new SessionPolicy();
        SessionSnapshot authenticated = new SessionSnapshot(
                "Example",
                UUID_VALUE,
                "MOJANG",
                true,
                true);

        assertEquals(SessionStatus.AUTHENTICATED, policy.evaluate(authenticated).getStatus());
        assertFalse(authenticated.toString().contains("access-token"));
    }

    @Test
    public void normalizesUndashedUuidWithoutHoldingCredential() {
        SessionSnapshot snapshot = new SessionSnapshot(
                "Example",
                "123456781234123492341234567890ab",
                "mojang",
                true,
                true);

        assertEquals(UUID_VALUE, snapshot.getUuid().get().toString());
        assertTrue(snapshot.isCredentialPresent());
    }
}
