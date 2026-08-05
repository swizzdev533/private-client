package client.privateclient.modules.api;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import client.privateclient.events.CoreEventBus;
import java.util.Collections;
import java.util.LinkedHashSet;
import java.util.Set;
import org.junit.Test;

public final class ModuleRegistryTest {
    @Test
    public void enablesDependenciesAndPreventsUnsafeDisable() throws Exception {
        CoreEventBus bus = new CoreEventBus((event, failure) -> {
        });
        ModuleContext context = new ModuleContext(bus);
        TestModule base = new TestModule("base-info", Collections.<String>emptySet(), context, false);
        TestModule child = new TestModule(
                "child-info",
                Collections.singleton("base-info"),
                context,
                false);
        ModuleRegistry registry = new ModuleRegistry(bus);
        registry.register(base);
        registry.register(child);

        registry.enable("child-info");

        assertTrue(base.isEnabled());
        assertTrue(child.isEnabled());
        try {
            registry.disable("base-info");
            fail("Expected dependency protection");
        } catch (ModuleActivationException expected) {
            assertTrue(expected.getMessage().contains("required"));
        }
    }

    @Test
    public void failedActivationRollsBackDependencies() {
        CoreEventBus bus = new CoreEventBus((event, failure) -> {
        });
        ModuleContext context = new ModuleContext(bus);
        TestModule base = new TestModule("base-info", Collections.<String>emptySet(), context, false);
        TestModule failing = new TestModule(
                "failing-info",
                Collections.singleton("base-info"),
                context,
                true);
        ModuleRegistry registry = new ModuleRegistry(bus);
        registry.register(base);
        registry.register(failing);

        try {
            registry.enable("failing-info");
            fail("Expected activation failure");
        } catch (ModuleActivationException expected) {
            assertFalse(base.isEnabled());
            assertFalse(failing.isEnabled());
        }
    }

    @Test
    public void rejectsCheatNamedModules() {
        CoreEventBus bus = new CoreEventBus((event, failure) -> {
        });
        ModuleContext context = new ModuleContext(bus);
        ModuleRegistry registry = new ModuleRegistry(bus);
        ClientModule forbidden = new AbstractClientModule(
                "combat-helper",
                "Aim assist",
                "Forbidden test fixture",
                ModuleCategory.INFORMATIONAL,
                false,
                Collections.<String>emptySet(),
                Collections.<String>emptySet(),
                context) {
        };

        try {
            registry.register(forbidden);
            fail("Expected safe module policy rejection");
        } catch (IllegalArgumentException expected) {
            assertTrue(expected.getMessage().contains("Forbidden"));
        }
    }

    private static final class TestModule extends AbstractClientModule {
        private final boolean failOnEnable;

        private TestModule(
                String id,
                Set<String> dependencies,
                ModuleContext context,
                boolean failOnEnable) {
            super(
                    id,
                    id,
                    "Test informational module",
                    ModuleCategory.INFORMATIONAL,
                    false,
                    new LinkedHashSet<String>(dependencies),
                    Collections.<String>emptySet(),
                    context);
            this.failOnEnable = failOnEnable;
        }

        @Override
        protected void onEnable(ModuleContext context) {
            if (failOnEnable) {
                throw new IllegalStateException("fixture failure");
            }
        }
    }
}
