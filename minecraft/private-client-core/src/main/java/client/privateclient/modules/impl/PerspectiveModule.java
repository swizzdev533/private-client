package client.privateclient.modules.impl;

import client.privateclient.modules.api.AbstractClientModule;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import java.util.Collections;

public final class PerspectiveModule extends AbstractClientModule {
    public static final String ID = "perspective";

    public PerspectiveModule(ModuleContext context) {
        super(
                ID,
                "Private Perspective",
                "Hold keybind (default Left Alt) for a 360-degree 3rd-person freelook camera without turning your player body.",
                ModuleCategory.COSMETIC,
                true,
                Collections.<String>emptySet(),
                Collections.<String>emptySet(),
                context);
    }
}
