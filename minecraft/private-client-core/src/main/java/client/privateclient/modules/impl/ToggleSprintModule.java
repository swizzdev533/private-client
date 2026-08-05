package client.privateclient.modules.impl;

import client.privateclient.modules.api.AbstractClientModule;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import java.util.Collections;

public final class ToggleSprintModule extends AbstractClientModule {
    public static final String ID = "sprinttoggle";

    public ToggleSprintModule(ModuleContext context) {
        super(
                ID,
                "Auto Sprint",
                "Automatically toggles and holds sprint when walking forward.",
                ModuleCategory.COSMETIC,
                true,
                Collections.<String>emptySet(),
                Collections.<String>emptySet(),
                context);
    }
}
