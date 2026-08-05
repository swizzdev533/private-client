package client.privateclient.util;

public final class FpsDisplayUtil {
    private FpsDisplayUtil() {
    }

    public static String format(int framesPerSecond) {
        return Math.max(0, framesPerSecond) + " FPS";
    }
}
