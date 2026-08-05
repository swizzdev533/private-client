package client.privateclient.logging;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public final class SecretRedactorTest {
    @Test
    public void redactsHeadersJsonFieldsAndQueryStyleSecrets() {
        String secret = "very.secret.token-value";
        String input = "Authorization: Bearer " + secret
                + "\n{\"access_token\":\"" + secret + "\",\"safe\":\"ok\"}"
                + "\nrefreshToken=" + secret
                + "\nCookie: SID=" + secret;

        String output = new SecretRedactor().redact(input);

        assertFalse(output.contains(secret));
        assertTrue(output.contains("[REDACTED]"));
        assertTrue(output.contains("\"safe\":\"ok\""));
    }

    @Test
    public void handlesNullWithoutThrowing() {
        assertTrue(new SecretRedactor().redact(null) == null);
    }
}
