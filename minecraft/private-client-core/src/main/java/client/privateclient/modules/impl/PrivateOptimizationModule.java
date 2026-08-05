package client.privateclient.modules.impl;

import client.privateclient.modules.api.AbstractClientModule;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import java.util.Collections;

public final class PrivateOptimizationModule extends AbstractClientModule {
    public static final String ID = "privateoptimization";

    public PrivateOptimizationModule(ModuleContext context) {
        super(
                ID,
                "Private Optimization Engine",
                "Consolidated performance optimizations (Entity Culling, Fast Math, Fast Font, Particle Culling, Nothirium, FoamFix memory cleanup).",
                ModuleCategory.PERFORMANCE,
                true,
                Collections.<String>emptySet(),
                Collections.<String>emptySet(),
                context);
    }
}
