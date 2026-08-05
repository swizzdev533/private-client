package client.privateclient.forge;

import client.privateclient.logging.SafeLogger;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import net.minecraft.client.gui.GuiScreen;

/** Opens the account selector from the required, pinned IAS mod without bundling its classes. */
final class AccountSwitcherScreenFactory {
    private static final String CONFIG_CLASS =
            "com.github.mrebhan.ingameaccountswitcher.tools.Config";
    private static final String SELECTOR_CLASS =
            "the_fireplace.ias.gui.GuiAccountSelector";

    private final String configClassName;
    private final String selectorClassName;

    AccountSwitcherScreenFactory() {
        this(CONFIG_CLASS, SELECTOR_CLASS);
    }

    AccountSwitcherScreenFactory(String configClassName, String selectorClassName) {
        this.configClassName = configClassName;
        this.selectorClassName = selectorClassName;
    }

    GuiScreen create(GuiScreen previousScreen, SafeLogger log) {
        try {
            initializeConfig();
            Class<?> selectorClass = Class.forName(selectorClassName);
            Constructor<?> constructor = selectorClass.getConstructor(GuiScreen.class);
            Object selector = constructor.newInstance(previousScreen);
            if (!(selector instanceof GuiScreen)) {
                throw new IllegalStateException("IAS selector is not a Minecraft GUI screen");
            }
            return (GuiScreen) selector;
        } catch (ReflectiveOperationException exception) {
            log.error("Could not open the required In-Game Account Switcher screen", exception);
            return previousScreen;
        } catch (LinkageError error) {
            log.error("Could not link the required In-Game Account Switcher screen", error);
            return previousScreen;
        }
    }

    private void initializeConfig() throws ReflectiveOperationException {
        Class<?> configClass = Class.forName(configClassName);
        Method getInstance = configClass.getMethod("getInstance");
        if (getInstance.invoke(null) == null) {
            configClass.getMethod("load").invoke(null);
        }
    }
}
