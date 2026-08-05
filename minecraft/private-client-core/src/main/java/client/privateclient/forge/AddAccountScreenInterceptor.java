package client.privateclient.forge;

import client.privateclient.gui.GuiAddOfflineAccount;
import client.privateclient.logging.SafeLogger;
import java.lang.reflect.Field;
import net.minecraft.client.gui.GuiScreen;

/**
 * Swaps the IAS "Add Account" screen for the Private Client offline-name form.
 *
 * <p>The upstream screen asks for a Mojang email and password. Private Client neither collects nor
 * stores passwords, so the whole screen is replaced instead of being partially hidden.
 */
public final class AddAccountScreenInterceptor {
    private static final String ADD_ACCOUNT_CLASS = "the_fireplace.ias.gui.GuiAddAccount";
    private static final String PREVIOUS_SCREEN_FIELD = "prev";

    private final String addAccountClassName;
    private final OfflineAccountRepository accounts;

    public AddAccountScreenInterceptor() {
        this(ADD_ACCOUNT_CLASS, new OfflineAccountRepository());
    }

    AddAccountScreenInterceptor(String addAccountClassName, OfflineAccountRepository accounts) {
        this.addAccountClassName = addAccountClassName;
        this.accounts = accounts;
    }

    /**
     * @return the replacement screen, or {@code null} when the opened screen is not the IAS
     *     "Add Account" screen.
     */
    public GuiScreen replace(GuiScreen gui, SafeLogger log) {
        if (gui == null || !addAccountClassName.equals(gui.getClass().getName())) {
            return null;
        }
        return new GuiAddOfflineAccount(previousScreen(gui, log), accounts, log);
    }

    private GuiScreen previousScreen(GuiScreen gui, SafeLogger log) {
        Class<?> current = gui.getClass();
        while (current != null) {
            try {
                Field field = current.getDeclaredField(PREVIOUS_SCREEN_FIELD);
                field.setAccessible(true);
                Object value = field.get(gui);
                return value instanceof GuiScreen ? (GuiScreen) value : null;
            } catch (NoSuchFieldException ignored) {
                current = current.getSuperclass();
            } catch (IllegalAccessException exception) {
                log.error("Could not read the previous account switcher screen", exception);
                return null;
            }
        }
        return null;
    }
}
