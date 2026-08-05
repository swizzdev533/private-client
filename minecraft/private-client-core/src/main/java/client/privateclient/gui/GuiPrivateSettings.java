package client.privateclient.gui;

import client.privateclient.config.ConfigStore;
import client.privateclient.config.CoreConfig;
import client.privateclient.discord.DiscordPresenceService;
import client.privateclient.modules.api.ModuleActivationException;
import client.privateclient.modules.api.ModuleRegistry;
import client.privateclient.modules.impl.NametagsModule;
import client.privateclient.modules.impl.FpsDisplayModule;
import client.privateclient.modules.impl.PerspectiveModule;
import client.privateclient.modules.impl.ToggleSprintModule;
import java.io.IOException;
import java.util.LinkedHashMap;
import java.util.Map;
import net.minecraft.client.gui.GuiButton;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.resources.I18n;

public final class GuiPrivateSettings extends GuiScreen {
    private final GuiScreen parentScreen;
    private final ConfigStore configStore;
    private final ModuleRegistry modules;
    private final DiscordPresenceService discordPresence;
    private CoreConfig currentConfig;

    private static final int BUTTON_DONE_ID = 200;
    private static final int BUTTON_STREAMER_MODE_ID = 100;
    private static final int BUTTON_PERSPECTIVE_ID = 101;
    private static final int BUTTON_NAMETAGS_ID = 102;
    private static final int BUTTON_SPRINT_ID = 103;
    private static final int BUTTON_FPS_DISPLAY_ID = 104;
    private static final int BUTTON_DISCORD_PRESENCE_ID = 105;

    public GuiPrivateSettings(
            GuiScreen parentScreen,
            ConfigStore configStore,
            CoreConfig currentConfig,
            DiscordPresenceService discordPresence,
            ModuleRegistry modules) {
        this.parentScreen = parentScreen;
        this.configStore = configStore;
        this.currentConfig = currentConfig;
        this.discordPresence = discordPresence;
        this.modules = modules;
    }

    @Override
    public void initGui() {
        this.buttonList.clear();
        int leftX = this.width / 2 - 155;
        int rightX = this.width / 2 + 5;
        int startY = this.height / 6 + 18;

        this.buttonList.add(new GuiButton(
                BUTTON_STREAMER_MODE_ID,
                leftX,
                startY,
                150,
                20,
                getStreamerModeText()));

        this.buttonList.add(new GuiButton(
                BUTTON_PERSPECTIVE_ID,
                rightX,
                startY,
                150,
                20,
                getModuleText(PerspectiveModule.ID, "Private Perspective")));

        this.buttonList.add(new GuiButton(
                BUTTON_NAMETAGS_ID,
                leftX,
                startY + 24,
                150,
                20,
                getModuleText(NametagsModule.ID, "Private Nametags")));

        this.buttonList.add(new GuiButton(
                BUTTON_SPRINT_ID,
                rightX,
                startY + 24,
                150,
                20,
                getModuleText(ToggleSprintModule.ID, "Auto Sprint")));

        this.buttonList.add(new GuiButton(
                BUTTON_FPS_DISPLAY_ID,
                leftX,
                startY + 48,
                150,
                20,
                getModuleText(FpsDisplayModule.ID, "FPS Display")));

        this.buttonList.add(new GuiButton(
                BUTTON_DISCORD_PRESENCE_ID,
                rightX,
                startY + 48,
                150,
                20,
                getDiscordPresenceText()));

        this.buttonList.add(new GuiButton(
                BUTTON_DONE_ID,
                this.width / 2 - 100,
                this.height / 6 + 96,
                200,
                20,
                I18n.format("gui.done")));
    }

    @Override
    protected void actionPerformed(GuiButton button) throws IOException {
        if (!button.enabled) {
            return;
        }

        if (button.id == BUTTON_DONE_ID) {
            this.mc.displayGuiScreen(this.parentScreen);
            return;
        }

        if (button.id == BUTTON_STREAMER_MODE_ID) {
            boolean nextState = !currentConfig.isStreamerModeEnabled();
            currentConfig = currentConfig.withStreamerMode(nextState);
            configStore.save(currentConfig);
            button.displayString = getStreamerModeText();
            return;
        }

        if (button.id == BUTTON_DISCORD_PRESENCE_ID) {
            currentConfig = currentConfig.withDiscordPresence(!currentConfig.isDiscordPresenceEnabled());
            configStore.save(currentConfig);
            discordPresence.setEnabled(currentConfig.isDiscordPresenceEnabled());
            button.displayString = getDiscordPresenceText();
            return;
        }

        String moduleId = null;
        String name = "";

        if (button.id == BUTTON_PERSPECTIVE_ID) {
            moduleId = PerspectiveModule.ID;
            name = "Private Perspective";
        } else if (button.id == BUTTON_NAMETAGS_ID) {
            moduleId = NametagsModule.ID;
            name = "Private Nametags";
        } else if (button.id == BUTTON_SPRINT_ID) {
            moduleId = ToggleSprintModule.ID;
            name = "Auto Sprint";
        } else if (button.id == BUTTON_FPS_DISPLAY_ID) {
            moduleId = FpsDisplayModule.ID;
            name = "FPS Display";
        }

        if (moduleId != null) {
            boolean currentState = modules.get(moduleId).isEnabled();
            try {
                if (currentState) {
                    modules.disable(moduleId);
                } else {
                    modules.enable(moduleId);
                }
            } catch (ModuleActivationException ignored) {
            }
            Map<String, Boolean> states = new LinkedHashMap<String, Boolean>(currentConfig.getModules());
            // Persist what the module actually ended up as, not what was
            // requested: a failed enable/disable would otherwise leave the
            // config permanently disagreeing with the running state.
            states.put(moduleId, modules.get(moduleId).isEnabled());
            currentConfig = currentConfig.withModuleStates(states);
            configStore.save(currentConfig);
            button.displayString = getModuleText(moduleId, name);
        }
    }

    @Override
    public void drawScreen(int mouseX, int mouseY, float partialTicks) {
        this.drawDefaultBackground();
        this.drawCenteredString(
                this.fontRendererObj,
                "Private Client Settings",
                this.width / 2,
                15,
                0xFFFFFF);
        super.drawScreen(mouseX, mouseY, partialTicks);
    }

    private String getStreamerModeText() {
        return "Streamer Mode: " + (currentConfig.isStreamerModeEnabled() ? "ON" : "OFF");
    }

    private String getModuleText(String moduleId, String name) {
        boolean enabled = modules.get(moduleId).isEnabled();
        return name + ": " + (enabled ? "ON" : "OFF");
    }

    private String getDiscordPresenceText() {
        return "Discord Status: " + (currentConfig.isDiscordPresenceEnabled() ? "ON" : "OFF");
    }
}
