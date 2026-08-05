package client.privateclient.modules.impl;

import client.privateclient.modules.api.AbstractClientModule;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import java.util.Collections;

public final class NametagsModule extends AbstractClientModule {
    public static final String ID = "nametags";

    public NametagsModule(ModuleContext context) {
        super(
                ID,
                "Private Nametags",
                "Displays your own nametag in 3rd-person view and inventory screen preview.",
                ModuleCategory.COSMETIC,
                true,
                Collections.<String>emptySet(),
                Collections.<String>emptySet(),
                context);
    }
}
