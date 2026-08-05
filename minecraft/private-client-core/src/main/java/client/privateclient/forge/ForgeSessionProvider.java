package client.privateclient.forge;

import client.privateclient.auth.SessionProvider;
import client.privateclient.auth.SessionSnapshot;
import com.mojang.authlib.GameProfile;
import net.minecraft.client.Minecraft;
import net.minecraft.util.Session;

public final class ForgeSessionProvider implements SessionProvider {
    @Override
    public SessionSnapshot capture() {
        Session session = Minecraft.getMinecraft().getSession();
        if (session == null) {
            return SessionSnapshot.missing();
        }

        GameProfile profile = session.getProfile();
        boolean profileIdentityPresent = profile != null && profile.getId() != null;
        String uuid = profileIdentityPresent
                ? profile.getId().toString()
                : session.getPlayerID();

        // The credential is deliberately reduced to a boolean immediately. The raw
        // token is never copied into a domain object, event, profile or log.
        String credential = session.getToken();
        boolean credentialPresent = credential != null
                && credential.trim().length() >= 16
                && !"0".equals(credential.trim())
                && !"-".equals(credential.trim());

        String sessionType = session.getSessionType() == null
                ? ""
                : session.getSessionType().name();
        return new SessionSnapshot(
                session.getUsername(),
                uuid,
                sessionType,
                credentialPresent,
                profileIdentityPresent);
    }
}
