package client.privateclient.modules.impl;

import client.privateclient.modules.api.AbstractClientModule;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import java.util.Collections;

public final class ScoreboardModule extends AbstractClientModule {
    public static final String ID = "scoreboardmod";

    public ScoreboardModule(ModuleContext context) {
        super(
                ID,
                "Scoreboard Customization",
                "Customizes the sidebar scoreboard display by hiding score numbers, tweaking background opacity, and formatting text.",
                ModuleCategory.INFORMATIONAL,
                true,
                Collections.<String>emptySet(),
                Collections.<String>emptySet(),
                context);
    }
}
