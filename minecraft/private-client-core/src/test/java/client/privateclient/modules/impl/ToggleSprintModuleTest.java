package client.privateclient.modules.impl;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import client.privateclient.events.CoreEventBus;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import client.privateclient.modules.api.ModuleRegistry;
import client.privateclient.modules.api.SafeModulePolicy;
import org.junit.Test;

public final class ToggleSprintModuleTest {
    @Test
    public void registersAndPassesSafeModulePolicy() throws Exception {
        CoreEventBus bus = new CoreEventBus((event, failure) -> {
        });
        ModuleContext context = new ModuleContext(bus);
        ToggleSprintModule module = new ToggleSprintModule(context);

        SafeModulePolicy.verify(module);
        assertEquals("sprinttoggle", module.getId());
        assertEquals("Auto Sprint", module.getName());
        assertEquals(ModuleCategory.COSMETIC, module.getCategory());
        assertTrue(module.isDefaultEnabled());

        ModuleRegistry registry = new ModuleRegistry(bus);
        registry.register(module);
        registry.enable("sprinttoggle");
        assertTrue(module.isEnabled());
    }
}
