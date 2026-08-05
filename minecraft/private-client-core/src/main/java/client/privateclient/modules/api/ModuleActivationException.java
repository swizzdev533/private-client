package client.privateclient.modules.api;

public final class ModuleActivationException extends Exception {
    private static final long serialVersionUID = 1L;

    public ModuleActivationException(String message) {
        super(message);
    }

    public ModuleActivationException(String message, Throwable cause) {
        super(message, cause);
    }
}
