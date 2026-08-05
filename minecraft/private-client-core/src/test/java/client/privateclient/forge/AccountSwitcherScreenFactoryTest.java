package client.privateclient.forge;

import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertTrue;

import client.privateclient.logging.SafeLogger;
import net.minecraft.client.gui.GuiScreen;
import org.apache.logging.log4j.LogManager;
import org.junit.Before;
import org.junit.Test;

public final class AccountSwitcherScreenFactoryTest {
    private final SafeLogger log = new SafeLogger(LogManager.getLogger(getClass()));

    @Before
    public void resetConfig() {
        FakeConfig.instance = null;
        FakeConfig.loaded = false;
    }

    @Test
    public void loadsIasConfigAndCreatesSelectorWithMainMenuAsPreviousScreen() {
        GuiScreen mainMenu = new GuiScreen() { };
        GuiScreen result = factory(FakeConfig.class, FakeSelector.class).create(mainMenu, log);

        assertTrue(FakeConfig.loaded);
        assertTrue(result instanceof FakeSelector);
        assertSame(mainMenu, ((FakeSelector) result).previous);
    }

    @Test
    public void keepsMainMenuAvailableWhenIasCannotBeLoaded() {
        GuiScreen mainMenu = new GuiScreen() { };
        GuiScreen result = new AccountSwitcherScreenFactory(
                "missing.ias.Config", "missing.ias.GuiAccountSelector").create(mainMenu, log);

        assertSame(mainMenu, result);
    }

    private static AccountSwitcherScreenFactory factory(
            Class<?> configClass, Class<?> selectorClass) {
        return new AccountSwitcherScreenFactory(configClass.getName(), selectorClass.getName());
    }

    public static final class FakeConfig {
        private static FakeConfig instance;
        private static boolean loaded;

        public static FakeConfig getInstance() {
            return instance;
        }

        public static void load() {
            loaded = true;
            instance = new FakeConfig();
        }
    }

    public static final class FakeSelector extends GuiScreen {
        private final GuiScreen previous;

        public FakeSelector(GuiScreen previous) {
            this.previous = previous;
        }
    }
}
