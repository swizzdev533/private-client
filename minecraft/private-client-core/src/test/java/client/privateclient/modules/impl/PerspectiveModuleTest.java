package client.privateclient.modules.impl;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

import client.privateclient.events.CoreEventBus;
import client.privateclient.modules.api.ModuleCategory;
import client.privateclient.modules.api.ModuleContext;
import client.privateclient.modules.api.ModuleRegistry;
import org.junit.Test;

public class PerspectiveModuleTest {

    @Test
    public void testPerspectiveModuleMetadataAndRegistration() throws Exception {
        CoreEventBus eventBus = new CoreEventBus((event, failure) -> {});
        ModuleContext context = new ModuleContext(eventBus);
        PerspectiveModule module = new PerspectiveModule(context);

        assertEquals("perspective", module.getId());
        assertEquals("Private Perspective", module.getName());
        assertEquals(ModuleCategory.COSMETIC, module.getCategory());
        assertTrue(module.isDefaultEnabled());

        ModuleRegistry registry = new ModuleRegistry(eventBus);
        registry.register(module);
        assertNotNull(registry.get("perspective"));
        assertFalse(module.isEnabled());

        registry.enable("perspective");
        assertTrue(module.isEnabled());

        registry.disable("perspective");
        assertFalse(module.isEnabled());
    }
}
