package client.privateclient.gui;

import client.privateclient.forge.OfflineAccountRepository;
import client.privateclient.logging.SafeLogger;
import java.io.IOException;
import java.lang.reflect.Constructor;
import net.minecraft.client.gui.GuiButton;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.gui.GuiTextField;
import org.lwjgl.input.Keyboard;

/**
 * Replaces the IAS "Add Account" screen with an offline-name-only form.
 *
 * <p>Private Client never collects a Mojang password, so the password field is gone. Microsoft
 * accounts still go through the IAS Microsoft flow, which never exposes credentials to us.
 */
public final class GuiAddOfflineAccount extends GuiScreen {
    private static final String MICROSOFT_SCREEN_CLASS = "ru.vidtu.iasfork.msauth.MSAuthScreen";

    private static final int BUTTON_ADD_ID = 0;
    private static final int BUTTON_CANCEL_ID = 1;
    private static final int BUTTON_MICROSOFT_ID = 13;

    private static final int NAME_FIELD_ID = 0;
    private static final int NAME_FIELD_WIDTH = 200;
    private static final int NAME_FIELD_HEIGHT = 20;
    private static final int NAME_MAX_LENGTH = 16;

    private final GuiScreen parentScreen;
    private final OfflineAccountRepository accounts;
    private final SafeLogger log;

    private GuiTextField nameField;
    private GuiButton addButton;
    private String status = "";

    public GuiAddOfflineAccount(
            GuiScreen parentScreen, OfflineAccountRepository accounts, SafeLogger log) {
        this.parentScreen = parentScreen;
        this.accounts = accounts;
        this.log = log;
    }

    @Override
    public void initGui() {
        Keyboard.enableRepeatEvents(true);
        this.buttonList.clear();

        int fieldX = this.width / 2 - 40;
        int fieldY = this.height / 4 + 24;
        this.nameField = new GuiTextField(
                NAME_FIELD_ID,
                this.fontRendererObj,
                fieldX,
                fieldY,
                NAME_FIELD_WIDTH,
                NAME_FIELD_HEIGHT);
        this.nameField.setMaxStringLength(NAME_MAX_LENGTH);
        this.nameField.setFocused(true);

        this.addButton = new GuiButton(
                BUTTON_ADD_ID,
                this.width / 2 - 154,
                this.height - 28,
                150,
                20,
                "Add Account");
        this.addButton.enabled = false;
        this.buttonList.add(this.addButton);
        this.buttonList.add(new GuiButton(
                BUTTON_CANCEL_ID,
                this.width / 2 + 4,
                this.height - 28,
                150,
                20,
                "Cancel"));
        this.buttonList.add(new GuiButton(
                BUTTON_MICROSOFT_ID,
                this.width / 2 - 60,
                this.height / 3 * 2,
                120,
                20,
                "Microsoft (Beta)"));
    }

    @Override
    public void onGuiClosed() {
        Keyboard.enableRepeatEvents(false);
    }

    @Override
    public void updateScreen() {
        if (this.nameField != null) {
            this.nameField.updateCursorCounter();
        }
        if (this.addButton != null) {
            this.addButton.enabled = OfflineAccountRepository.isValidName(currentName());
        }
    }

    @Override
    public void drawScreen(int mouseX, int mouseY, float partialTicks) {
        drawDefaultBackground();
        drawCenteredString(this.fontRendererObj, "Add Account", this.width / 2, 20, 0xFFFFFF);
        String label = "Offline name:";
        this.fontRendererObj.drawString(
                label,
                this.width / 2 - 46 - this.fontRendererObj.getStringWidth(label),
                this.height / 4 + 30,
                0xA0A0A0);
        if (this.nameField != null) {
            this.nameField.drawTextBox();
        }
        if (!this.status.isEmpty()) {
            drawCenteredString(
                    this.fontRendererObj, this.status, this.width / 2, this.height / 4 + 52, 0xFF5555);
        }
        super.drawScreen(mouseX, mouseY, partialTicks);
    }

    @Override
    protected void mouseClicked(int mouseX, int mouseY, int mouseButton) throws IOException {
        super.mouseClicked(mouseX, mouseY, mouseButton);
        if (this.nameField != null) {
            this.nameField.mouseClicked(mouseX, mouseY, mouseButton);
        }
    }

    @Override
    protected void keyTyped(char typedChar, int keyCode) throws IOException {
        if (keyCode == Keyboard.KEY_ESCAPE) {
            back();
            return;
        }
        if (keyCode == Keyboard.KEY_RETURN || keyCode == Keyboard.KEY_NUMPADENTER) {
            addAccount();
            return;
        }
        if (this.nameField != null) {
            this.nameField.textboxKeyTyped(typedChar, keyCode);
        }
        this.status = "";
    }

    @Override
    protected void actionPerformed(GuiButton button) throws IOException {
        if (!button.enabled) {
            return;
        }
        if (button.id == BUTTON_ADD_ID) {
            addAccount();
        } else if (button.id == BUTTON_CANCEL_ID) {
            back();
        } else if (button.id == BUTTON_MICROSOFT_ID) {
            openMicrosoftLogin();
        }
    }

    private void addAccount() {
        String name = currentName();
        if (!OfflineAccountRepository.isValidName(name)) {
            this.status = "Offline name: 3-16 characters (A-Z, 0-9, _)";
            return;
        }
        try {
            if (!accounts.add(name)) {
                this.status = "That account already exists";
                return;
            }
        } catch (ReflectiveOperationException exception) {
            log.error("Could not add an offline account to the account switcher", exception);
            this.status = "Could not save the account";
            return;
        } catch (LinkageError error) {
            log.error("Could not link the account switcher storage", error);
            this.status = "Could not save the account";
            return;
        }
        back();
    }

    private void openMicrosoftLogin() {
        try {
            Class<?> screenClass = Class.forName(MICROSOFT_SCREEN_CLASS);
            Constructor<?> constructor = screenClass.getConstructor(GuiScreen.class);
            Object screen = constructor.newInstance(this);
            if (screen instanceof GuiScreen) {
                this.mc.displayGuiScreen((GuiScreen) screen);
                return;
            }
            this.status = "Microsoft sign-in is unavailable";
        } catch (ReflectiveOperationException exception) {
            log.error("Could not open the Microsoft login screen", exception);
            this.status = "Microsoft sign-in is unavailable";
        } catch (LinkageError error) {
            log.error("Could not link the Microsoft login screen", error);
            this.status = "Microsoft sign-in is unavailable";
        }
    }

    private void back() {
        this.mc.displayGuiScreen(parentScreen);
    }

    private String currentName() {
        return this.nameField == null ? "" : this.nameField.getText().trim();
    }
}
