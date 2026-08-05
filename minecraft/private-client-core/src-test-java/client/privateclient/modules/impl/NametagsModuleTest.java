package client.privateclient.modules.impl;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import client.privateclient.events.CoreEventBus;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import client.privateclient.modules.api.ModuleRegistry;
import client.privateclient.modules.api.SafeModulePolicy;
import org.junit.Test;

public final class NametagsModuleTest {
    @Test
    public void registersAndPassesSafeModulePolicy() throws Exception {
        CoreEventBus bus = new CoreEventBus((event, failure) -> {
        });
        ModuleContext context = new ModuleContext(bus);
        NametagsModule module = new NametagsModule(context);

        SafeModulePolicy.verify(module);
        assertEquals("nametags", module.getId());
        assertEquals("Private Nametags", module.getName());
        assertEquals(ModuleCategory.COSMETIC, module.getCategory());
        assertTrue(module.isDefaultEnabled());

        ModuleRegistry registry = new ModuleRegistry(bus);
        registry.register(module);
        registry.enable("nametags");
        assertTrue(module.isEnabled());
    }
}
