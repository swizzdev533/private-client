package client.privateclient.auth;

public enum SessionStatus {
    AUTHENTICATED,
    MISSING_SESSION,
    INVALID_USERNAME,
    INVALID_UUID,
    OFFLINE_SESSION_TYPE,
    MISSING_CREDENTIAL,
    MISSING_PROFILE_IDENTITY
}
