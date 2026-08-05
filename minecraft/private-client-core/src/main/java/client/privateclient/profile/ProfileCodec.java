package client.privateclient.profile;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.time.Instant;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.UUID;

public final class ProfileCodec {
    private static final Set<String> ALLOWED_FIELDS = new HashSet<String>(Arrays.asList(
            "schemaVersion",
            "username",
            "uuid",
            "skinModel",
            "skinPath",
            "updatedAt"
    ));

    private final Gson gson = new GsonBuilder()
            .disableHtmlEscaping()
            .setPrettyPrinting()
            .create();

    public String encode(PlayerProfile profile) {
        JsonObject root = new JsonObject();
        root.addProperty("schemaVersion", profile.getSchemaVersion());
        root.addProperty("username", profile.getUsername());
        root.addProperty("uuid", profile.getUuid().toString());
        root.addProperty("skinModel", profile.getSkinModel().getSerializedName());
        root.addProperty("skinPath", profile.getSkinPath());
        root.addProperty("updatedAt", profile.getUpdatedAt().toString());
        return gson.toJson(root) + System.lineSeparator();
    }

    public PlayerProfile decode(String json) {
        JsonElement element = new JsonParser().parse(json);
        if (!element.isJsonObject()) {
            throw new IllegalArgumentException("Profile root must be an object");
        }
        JsonObject root = element.getAsJsonObject();
        Set<String> fields = new HashSet<String>();
        for (Map.Entry<String, JsonElement> entry : root.entrySet()) {
            fields.add(entry.getKey());
        }
        if (!ALLOWED_FIELDS.equals(fields)) {
            throw new IllegalArgumentException("Profile contains missing or forbidden fields");
        }
        return new PlayerProfile(
                root.get("schemaVersion").getAsInt(),
                root.get("username").getAsString(),
                UUID.fromString(root.get("uuid").getAsString()),
                SkinModel.fromSerializedName(root.get("skinModel").getAsString()),
                root.get("skinPath").getAsString(),
                Instant.parse(root.get("updatedAt").getAsString()));
    }
}
