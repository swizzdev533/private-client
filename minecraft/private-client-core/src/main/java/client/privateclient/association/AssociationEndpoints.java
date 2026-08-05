package client.privateclient.association;

/**
 * Pinned association API host and paths. Network policy must list this host
 * before production enablement.
 */
public final class AssociationEndpoints {
    /** Production Vercel host until a custom domain is attached. */
    public static final String HOST = "private-client-association.vercel.app";
    public static final String PRESENCE_PATH = "/api/v1/presence";
    public static final String PRESENCE_URL = "https://" + HOST + PRESENCE_PATH;
    public static final String CLIENT_VERSION = "1.0.0";
    public static final int SCHEMA_VERSION = 1;

    private AssociationEndpoints() {
    }
}
