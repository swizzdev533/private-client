package client.privateclient.forge;

import client.privateclient.auth.SessionObserver;
import java.io.IOException;
import net.minecraft.client.gui.GuiButton;
import net.minecraft.client.gui.GuiMainMenu;
import net.minecraft.client.gui.GuiMultiplayer;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.resources.I18n;

public final class SessionRequiredScreen extends GuiScreen {
    private static final int BUTTON_BACK = 0;
    private static final int BUTTON_RETRY = 1;

    private final SessionObserver sessionObserver;
    private String statusLine = "";

    public SessionRequiredScreen(SessionObserver sessionObserver) {
        this.sessionObserver = sessionObserver;
    }

    @Override
    public void initGui() {
        buttonList.clear();
        int centerY = height / 2;
        buttonList.add(new GuiButton(
                BUTTON_RETRY,
                width / 2 - 100,
                centerY + 36,
                I18n.format("privateclientcore.guard.retry")));
        buttonList.add(new GuiButton(
                BUTTON_BACK,
                width / 2 - 100,
                centerY + 62,
                I18n.format("privateclientcore.guard.back")));
    }

    @Override
    protected void actionPerformed(GuiButton button) throws IOException {
        if (button.id == BUTTON_BACK) {
            mc.displayGuiScreen(new GuiMainMenu());
            return;
        }
        if (button.id == BUTTON_RETRY) {
            try {
                sessionObserver.refresh();
                if (sessionObserver.getValidation().isAuthenticated()) {
                    mc.displayGuiScreen(new GuiMultiplayer(new GuiMainMenu()));
                } else {
                    statusLine = I18n.format("privateclientcore.guard.line1");
                }
            } catch (IOException exception) {
                statusLine = I18n.format("privateclientcore.guard.line1");
            }
        }
    }

    @Override
    public void drawScreen(int mouseX, int mouseY, float partialTicks) {
        drawDefaultBackground();
        int centerY = height / 2;
        drawCenteredString(
                fontRendererObj,
                I18n.format("privateclientcore.guard.title"),
                width / 2,
                centerY - 50,
                0xFFF2F2F2);
        drawCenteredString(
                fontRendererObj,
                I18n.format("privateclientcore.guard.line1"),
                width / 2,
                centerY - 22,
                0xFFD0D0D0);
        drawCenteredString(
                fontRendererObj,
                I18n.format("privateclientcore.guard.line2"),
                width / 2,
                centerY - 8,
                0xFFAAAAAA);
        if (!statusLine.isEmpty()) {
            drawCenteredString(fontRendererObj, statusLine, width / 2, centerY + 14, 0xFFC8C8C8);
        }
        super.drawScreen(mouseX, mouseY, partialTicks);
    }

    @Override
    public boolean doesGuiPauseGame() {
        return false;
    }
}
