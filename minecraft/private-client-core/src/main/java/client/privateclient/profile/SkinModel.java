package client.privateclient.profile;

public enum SkinModel {
    CLASSIC("classic"),
    SLIM("slim");

    private final String serializedName;

    SkinModel(String serializedName) {
        this.serializedName = serializedName;
    }

    public String getSerializedName() {
        return serializedName;
    }

    public static SkinModel fromSerializedName(String value) {
        for (SkinModel model : values()) {
            if (model.serializedName.equalsIgnoreCase(value)) {
                return model;
            }
        }
        throw new IllegalArgumentException("Unsupported skin model");
    }
}
