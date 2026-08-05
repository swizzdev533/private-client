package client.privateclient.modules.impl;

import client.privateclient.modules.api.AbstractClientModule;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import java.util.Collections;

public final class ItemPhysicsModule extends AbstractClientModule {
    public static final String ID = "itemphysics";

    public ItemPhysicsModule(ModuleContext context) {
        super(
                ID,
                "3D Item Physics",
                "Enhances dropped item rendering with 3D drop physics and realistic ground collision.",
                ModuleCategory.COSMETIC,
                true,
                Collections.<String>emptySet(),
                Collections.<String>emptySet(),
                context);
    }
}
