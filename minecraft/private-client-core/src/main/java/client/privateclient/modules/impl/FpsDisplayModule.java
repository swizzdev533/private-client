package client.privateclient.modules.impl;

import client.privateclient.modules.api.AbstractClientModule;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import java.util.Collections;

public final class FpsDisplayModule extends AbstractClientModule {
    public static final String ID = "fps-display";

    public FpsDisplayModule(ModuleContext context) {
        super(
                ID,
                "FPS Display",
                "Displays the current frame rate in the top-left corner.",
                ModuleCategory.INFORMATIONAL,
                false,
                Collections.<String>emptySet(),
                Collections.<String>emptySet(),
                context);
    }
}
