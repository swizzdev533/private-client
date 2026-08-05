package client.privateclient.logging;

import org.apache.logging.log4j.Logger;

public final class SafeLogger {
    private final Logger delegate;
    private final SecretRedactor redactor;

    public SafeLogger(Logger delegate) {
        if (delegate == null) {
            throw new IllegalArgumentException("Logger is required");
        }
        this.delegate = delegate;
        this.redactor = new SecretRedactor();
    }

    public void info(String message) {
        delegate.info(redactor.redact(message));
    }

    public void warn(String message) {
        delegate.warn(redactor.redact(message));
    }

    public void error(String message, Throwable failure) {
        String failureType = failure == null ? "unknown" : failure.getClass().getSimpleName();
        String failureMessage = failure == null ? "" : redactor.redact(failure.getMessage());
        delegate.error(redactor.redact(message)
                + " ["
                + failureType
                + (failureMessage == null || failureMessage.isEmpty() ? "" : ": " + failureMessage)
                + "]");
    }
}
