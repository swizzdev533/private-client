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

public class NametagsModuleTest {

    @Test
    public void testNametagsModuleMetadataAndRegistration() throws Exception {
        CoreEventBus eventBus = new CoreEventBus((event, failure) -> {});
        ModuleContext context = new ModuleContext(eventBus);
        NametagsModule module = new NametagsModule(context);

        assertEquals("nametags", module.getId());
        assertEquals("Private Nametags", module.getName());
        assertEquals(ModuleCategory.COSMETIC, module.getCategory());
        assertTrue(module.isDefaultEnabled());

        ModuleRegistry registry = new ModuleRegistry(eventBus);
        registry.register(module);
        assertNotNull(registry.get("nametags"));
        assertFalse(module.isEnabled());

        registry.enable("nametags");
        assertTrue(module.isEnabled());

        registry.disable("nametags");
        assertFalse(module.isEnabled());
    }
}
