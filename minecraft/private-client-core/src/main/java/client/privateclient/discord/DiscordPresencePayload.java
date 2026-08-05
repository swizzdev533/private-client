package client.privateclient.discord;

import com.google.gson.JsonObject;

final class DiscordPresencePayload {
    private DiscordPresencePayload() {
    }

    static String handshake(String applicationId) {
        JsonObject root = new JsonObject();
        root.addProperty("v", 1);
        root.addProperty("client_id", applicationId);
        return root.toString();
    }

    static String activity(long processId, long startedAtSeconds, String largeImage) {
        JsonObject activity = new JsonObject();
        activity.addProperty("details", "Playing Private Client");
        activity.addProperty("state", "Minecraft 1.8.9");

        JsonObject timestamps = new JsonObject();
        timestamps.addProperty("start", startedAtSeconds);
        activity.add("timestamps", timestamps);

        JsonObject assets = new JsonObject();
        assets.addProperty("large_image", largeImage);
        assets.addProperty("large_text", "Private Client 1.0.0");
        activity.add("assets", assets);

        JsonObject args = new JsonObject();
        args.addProperty("pid", processId);
        args.add("activity", activity);

        JsonObject root = new JsonObject();
        root.addProperty("cmd", "SET_ACTIVITY");
        root.add("args", args);
        root.addProperty("nonce", Long.toHexString(System.nanoTime()));
        return root.toString();
    }
}
