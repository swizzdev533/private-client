package client.privateclient.forge;

import client.privateclient.association.AssociationService;
import client.privateclient.auth.SessionObserver;
import client.privateclient.badge.PrivetBadgeRenderer;
import client.privateclient.badge.TabListBadgeOverlay;
import client.privateclient.branding.WindowTitleTransformer;
import client.privateclient.config.ConfigStore;
import client.privateclient.config.CoreConfig;
import client.privateclient.discord.DiscordPresenceService;
import client.privateclient.events.CoreEvent;
import client.privateclient.events.CoreEventBus;
import client.privateclient.events.CoreEventType;
import client.privateclient.gui.GuiPrivateSettings;
import client.privateclient.gui.GuiPrivateMainMenu;
import client.privateclient.logging.SafeLogger;
import client.privateclient.modules.api.ModuleRegistry;
import client.privateclient.modules.impl.ItemPhysicsModule;
import client.privateclient.modules.impl.FpsDisplayModule;
import client.privateclient.modules.impl.NametagsModule;
import client.privateclient.modules.impl.PerspectiveModule;
import client.privateclient.modules.impl.PrivateOptimizationModule;
import client.privateclient.modules.impl.ScoreboardModule;
import client.privateclient.modules.impl.ToggleSprintModule;
import client.privateclient.util.StreamerModeUtil;
import client.privateclient.util.FpsDisplayUtil;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.SocketAddress;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.Comparator;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.client.network.NetworkPlayerInfo;
import net.minecraft.client.gui.GuiButton;
import net.minecraft.client.gui.GuiIngameMenu;
import net.minecraft.client.gui.GuiMainMenu;
import net.minecraft.client.gui.GuiOptions;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.WorldRenderer;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.client.settings.KeyBinding;
import net.minecraft.entity.EntityLivingBase;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraft.util.ChatComponentText;
import net.minecraft.util.IChatComponent;
import net.minecraft.scoreboard.ScorePlayerTeam;
import net.minecraftforge.client.event.ClientChatReceivedEvent;
import net.minecraftforge.client.event.EntityViewRenderEvent;
import net.minecraftforge.client.event.GuiOpenEvent;
import net.minecraftforge.client.event.GuiScreenEvent;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.client.event.RenderHandEvent;
import net.minecraftforge.client.event.RenderLivingEvent;
import net.minecraftforge.client.event.RenderPlayerEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.event.world.WorldEvent;
import net.minecraftforge.fml.client.registry.ClientRegistry;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import net.minecraftforge.fml.common.network.FMLNetworkEvent;
import org.lwjgl.input.Keyboard;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.Display;

@SuppressWarnings("deprecation")
public final class ForgeClientHooks {
    private static final int SESSION_POLL_INTERVAL_TICKS = 20;
    private static final int PRIVATE_SETTINGS_BUTTON_ID = 999;

    private final SessionObserver sessionObserver;
    private final ModuleRegistry modules;
    private final CoreEventBus eventBus;
    private final ConfigStore configStore;
    private volatile CoreConfig config;
    private final DiscordPresenceService discordPresence;
    private final AssociationService associationService;
    private final TabListBadgeOverlay tabListBadgeOverlay;
    private final SafeLogger log;
    private final AccountSwitcherScreenFactory accountSwitcherScreens =
            new AccountSwitcherScreenFactory();
    private final AddAccountScreenInterceptor addAccountScreens =
            new AddAccountScreenInterceptor();

    private final KeyBinding toggleSprintKey = new KeyBinding(
            "Toggle Sprint",
            Keyboard.KEY_LCONTROL,
            "Private Client");

    private final KeyBinding perspectiveKey = new KeyBinding(
            "Private Perspective",
            Keyboard.KEY_LMENU,
            "Private Client");

    private boolean sprintToggled = true;
    private int ticksUntilSessionPoll;
    private final Map<NetworkPlayerInfo, IChatComponent> tabDisplayNameSnapshot =
            new IdentityHashMap<NetworkPlayerInfo, IChatComponent>();

    // Perspective freelook state
    private boolean perspectiveActive = false;
    // The account selector is a launch-time prompt only; later main menu visits stay on the menu.
    private boolean accountSwitcherPrompted = false;
    private float cameraYaw = 0.0F;
    private float cameraPitch = 0.0F;
    private float lockedPlayerYaw = 0.0F;
    private float lockedPlayerPitch = 0.0F;
    private int originalThirdPersonView = 0;

    public ForgeClientHooks(
            SessionObserver sessionObserver,
            ModuleRegistry modules,
            CoreEventBus eventBus,
            ConfigStore configStore,
            CoreConfig config,
            DiscordPresenceService discordPresence,
            AssociationService associationService,
            SafeLogger log) {
        this.sessionObserver = sessionObserver;
        this.modules = modules;
        this.eventBus = eventBus;
        this.configStore = configStore;
        this.config = config;
        this.discordPresence = discordPresence;
        this.associationService = associationService;
        this.tabListBadgeOverlay = new TabListBadgeOverlay(associationService);
        this.log = log;
    }

    public void register() {
        ensureWindowTitle();
        MinecraftForge.EVENT_BUS.register(this);
        FMLCommonHandler.instance().bus().register(this);
        ClientRegistry.registerKeyBinding(toggleSprintKey);
        ClientRegistry.registerKeyBinding(perspectiveKey);
    }

    public void unregister() {
        MinecraftForge.EVENT_BUS.unregister(this);
        FMLCommonHandler.instance().bus().unregister(this);
    }

    @SubscribeEvent
    public void onClientTick(TickEvent.ClientTickEvent event) {
        if (event.phase != TickEvent.Phase.END) {
            return;
        }
        ensureWindowTitle();
        eventBus.publish(CoreEvent.of(CoreEventType.TICK));

        Minecraft minecraft = Minecraft.getMinecraft();
        if (minecraft.thePlayer != null && minecraft.getCurrentServerData() != null) {
            associationService.updatePublishedIdentity(
                    minecraft.thePlayer.getGameProfile().getId(),
                    minecraft.thePlayer.getName());
        }
        if (ticksUntilSessionPoll <= 0) {
            refreshSession();
            ticksUntilSessionPoll = SESSION_POLL_INTERVAL_TICKS;
        } else {
            ticksUntilSessionPoll--;
        }

        if (toggleSprintKey.isPressed()) {
            sprintToggled = !sprintToggled;
        }

        if (modules.get(ToggleSprintModule.ID).isEnabled()
                && sprintToggled
                && minecraft.thePlayer != null
                && minecraft.currentScreen == null) {
            if (minecraft.gameSettings.keyBindForward.isKeyDown()
                    && !minecraft.thePlayer.isSneaking()
                    && !minecraft.thePlayer.isCollidedHorizontally
                    && minecraft.thePlayer.getFoodStats().getFoodLevel() > 6) {
                minecraft.thePlayer.setSprinting(true);
            }
        }

        // Handle Private Perspective freelook state
        if (modules.get(PerspectiveModule.ID).isEnabled() && minecraft.thePlayer != null) {
            if (perspectiveKey.isKeyDown()) {
                if (!perspectiveActive) {
                    perspectiveActive = true;
                    originalThirdPersonView = minecraft.gameSettings.thirdPersonView;
                    minecraft.gameSettings.thirdPersonView = 1;
                    cameraYaw = minecraft.thePlayer.rotationYaw + 180.0F;
                    cameraPitch = minecraft.thePlayer.rotationPitch;
                    lockedPlayerYaw = minecraft.thePlayer.rotationYaw;
                    lockedPlayerPitch = minecraft.thePlayer.rotationPitch;
                }
                // Lock player rotation angles so movement and body do NOT turn!
                lockPlayerRotation(minecraft.thePlayer);
            } else if (perspectiveActive) {
                perspectiveActive = false;
                minecraft.gameSettings.thirdPersonView = originalThirdPersonView;
            }
        } else if (perspectiveActive) {
            perspectiveActive = false;
            if (minecraft.gameSettings != null) {
                minecraft.gameSettings.thirdPersonView = originalThirdPersonView;
            }
        }
    }

    @SubscribeEvent
    public void onRenderGameOverlayPre(RenderGameOverlayEvent.Pre event) {
        if (perspectiveActive && event.type == RenderGameOverlayEvent.ElementType.CROSSHAIRS) {
            event.setCanceled(true);
        }
        if (config.isStreamerModeEnabled()
                && event.type == RenderGameOverlayEvent.ElementType.PLAYER_LIST) {
            maskTabListNames();
        }
    }

    @SubscribeEvent
    public void onRenderGameOverlayPost(RenderGameOverlayEvent.Post event) {
        if (event.type == RenderGameOverlayEvent.ElementType.PLAYER_LIST) {
            tabListBadgeOverlay.render(event);
            restoreTabListNames();
        }
    }

    @SubscribeEvent
    public void onClientConnected(FMLNetworkEvent.ClientConnectedToServerEvent event) {
        if (event.isLocal) {
            associationService.onDisconnected();
            return;
        }
        refreshSession();
        // Register both the typed server-list address and the resolved remote
        // socket. SRV names and raw IPs otherwise land in different presence buckets.
        ServerData server = Minecraft.getMinecraft().getCurrentServerData();
        if (server != null && server.serverIP != null && !server.serverIP.trim().isEmpty()) {
            registerAssociationEndpoint(server.serverIP.trim());
        }
        if (event.manager != null) {
            SocketAddress remote = event.manager.getRemoteAddress();
            if (remote instanceof InetSocketAddress) {
                InetSocketAddress inet = (InetSocketAddress) remote;
                String host = inet.getAddress() != null
                        ? inet.getAddress().getHostAddress()
                        : inet.getHostString();
                if (host != null && !host.isEmpty() && inet.getPort() > 0) {
                    associationService.registerServerEndpoint(host, inet.getPort());
                }
            }
        }
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.thePlayer != null) {
            associationService.updatePublishedIdentity(
                    mc.thePlayer.getGameProfile().getId(),
                    mc.thePlayer.getName());
        }
    }

    private void registerAssociationEndpoint(String serverIp) {
        String[] parts = serverIp.split(":");
        String host = parts[0];
        int port = 25565;
        if (parts.length > 1) {
            try {
                port = Integer.parseInt(parts[1]);
            } catch (NumberFormatException ignored) {
                port = 25565;
            }
        }
        associationService.registerServerEndpoint(host, port);
    }

    @SubscribeEvent
    public void onClientDisconnected(FMLNetworkEvent.ClientDisconnectionFromServerEvent event) {
        associationService.onDisconnected();
    }

    @SubscribeEvent
    public void onRenderTick(TickEvent.RenderTickEvent event) {
        Minecraft minecraft = Minecraft.getMinecraft();
        if (perspectiveActive
                && minecraft.inGameHasFocus
                && minecraft.currentScreen == null
                && minecraft.thePlayer != null) {
            lockPlayerRotation(minecraft.thePlayer);

            float sensitivity = minecraft.gameSettings.mouseSensitivity * 0.6F + 0.2F;
            float factor = sensitivity * sensitivity * sensitivity * 8.0F;
            float dx = minecraft.mouseHelper.deltaX * factor * 0.15F;
            float dy = minecraft.mouseHelper.deltaY * factor * 0.15F;

            cameraYaw += dx;
            cameraPitch -= dy;
            if (cameraPitch > 90.0F) {
                cameraPitch = 90.0F;
            }
            if (cameraPitch < -90.0F) {
                cameraPitch = -90.0F;
            }

            // Zero out mouseHelper deltas so player body does NOT turn!
            minecraft.mouseHelper.deltaX = 0;
            minecraft.mouseHelper.deltaY = 0;
        }
    }

    @SubscribeEvent
    public void onCameraSetup(EntityViewRenderEvent.CameraSetup event) {
        if (perspectiveActive) {
            event.yaw = cameraYaw;
            event.pitch = cameraPitch;
        }
    }

    @SubscribeEvent
    public void onRenderHand(RenderHandEvent event) {
        if (perspectiveActive) {
            event.setCanceled(true);
        }
    }

    @SubscribeEvent
    public void onRenderPlayerSpecialsPre(RenderPlayerEvent.Specials.Pre event) {
        // Always take over player nametags so PrivetBadge can render Lunar-style.
        if (event.entityPlayer != null) {
            event.setCanceled(true);
        }
    }

    @SubscribeEvent
    public void onRenderLivingSpecialsPre(RenderLivingEvent.Specials.Pre<EntityLivingBase> event) {
        if (event.entity instanceof EntityPlayer) {
            event.setCanceled(true);
        }
    }

    @SubscribeEvent
    public void onRenderPlayerPost(RenderPlayerEvent.Post event) {
        EntityPlayer player = event.entityPlayer;
        Minecraft mc = Minecraft.getMinecraft();
        if (player == null || mc == null || mc.thePlayer == null) {
            return;
        }

        // For thePlayer: nametag (with badge) only in 3rd person (F5), Freelook, or GUI.
        if (player == mc.thePlayer) {
            boolean thirdPerson = mc.gameSettings.thirdPersonView != 0;
            boolean inGui = mc.currentScreen != null;
            if (!thirdPerson && !perspectiveActive && !inGui) {
                return;
            }
        }

        String name = StreamerModeUtil.sanitizeName(
                player.getDisplayName().getFormattedText(),
                config.isStreamerModeEnabled());
        // Local player always gets PrivetBadge; peers come from association cache.
        boolean showBadge = player == mc.thePlayer
                || associationService.shouldShowBadge(
                        player.getGameProfile().getId(),
                        player.getName());
        renderCustomNametag(player, name, event.x, event.y, event.z, showBadge);
    }

    @SubscribeEvent
    public void onGuiOpen(GuiOpenEvent event) {
        ensureWindowTitle();
        if (event.gui != null && event.gui.getClass() == GuiMainMenu.class) {
            try {
                GuiScreen mainMenu = new GuiPrivateMainMenu(sessionObserver);
                if (accountSwitcherPrompted) {
                    event.gui = mainMenu;
                } else {
                    accountSwitcherPrompted = true;
                    event.gui = accountSwitcherScreens.create(mainMenu, log);
                }
            } catch (Throwable exception) {
                // Never take down game init for a menu skin/UI failure.
                log.error("Could not open Private Client main menu; keeping vanilla", exception);
            }
        }
        try {
            GuiScreen offlineAddAccount = addAccountScreens.replace(event.gui, log);
            if (offlineAddAccount != null) {
                event.gui = offlineAddAccount;
            }
        } catch (Throwable exception) {
            // A broken swap must never block the account switcher.
            log.error("Could not open the Private Client add account screen", exception);
        }
        try {
            config = configStore.load().getConfig();
            discordPresence.setEnabled(config.isDiscordPresenceEnabled());
        } catch (IOException exception) {
            log.error("Could not reload Private Client settings", exception);
        }
    }

    @SubscribeEvent
    public void onChatReceived(ClientChatReceivedEvent event) {
        if (!config.isStreamerModeEnabled() || event.message == null) {
            return;
        }
        String unformatted = event.message.getUnformattedText();
        String sanitized = StreamerModeUtil.sanitizeKnownNames(
                unformatted,
                true,
                getKnownPlayerNames());
        event.message = new ChatComponentText(sanitized);
    }

    @SubscribeEvent
    public void onInitGui(GuiScreenEvent.InitGuiEvent.Post event) {
        if (event.gui == null) {
            return;
        }
        Class<?> clazz = event.gui.getClass();
        if (clazz == GuiOptions.class || clazz == GuiIngameMenu.class) {
            event.buttonList.add(new GuiButton(
                    PRIVATE_SETTINGS_BUTTON_ID,
                    event.gui.width - 110,
                    6,
                    100,
                    20,
                    "Private Settings"));
        }
    }

    @SubscribeEvent
    public void onActionPerformed(GuiScreenEvent.ActionPerformedEvent.Pre event) {
        // The id alone is not enough: this fires for every screen, so another
        // mod's button that happens to use the same id would open our settings
        // and have its own action cancelled. Gate on the screens we added it to.
        if (event.button.id == PRIVATE_SETTINGS_BUTTON_ID && event.gui != null
                && (event.gui.getClass() == GuiOptions.class
                        || event.gui.getClass() == GuiIngameMenu.class)) {
            Minecraft minecraft = Minecraft.getMinecraft();
            minecraft.displayGuiScreen(new GuiPrivateSettings(
                    event.gui,
                    configStore,
                    config,
                    discordPresence,
                    modules));
            event.setCanceled(true);
        }
    }

    @SubscribeEvent
    public void onRender(RenderGameOverlayEvent.Post event) {
        if (event.type != RenderGameOverlayEvent.ElementType.TEXT) {
            return;
        }
        eventBus.publish(CoreEvent.of(CoreEventType.RENDER));
        renderInformationalHud();
    }

    @SubscribeEvent
    public void onWorldLoad(WorldEvent.Load event) {
        if (event.world.isRemote) {
            eventBus.publish(CoreEvent.of(CoreEventType.WORLD_LOAD));
        }
    }

    @SubscribeEvent
    public void onWorldUnload(WorldEvent.Unload event) {
        if (event.world.isRemote) {
            eventBus.publish(CoreEvent.of(CoreEventType.WORLD_UNLOAD));
        }
    }

    private void lockPlayerRotation(EntityPlayer player) {
        if (player == null) {
            return;
        }
        player.rotationYaw = lockedPlayerYaw;
        player.rotationPitch = lockedPlayerPitch;
        player.prevRotationYaw = lockedPlayerYaw;
        player.prevRotationPitch = lockedPlayerPitch;
        player.renderYawOffset = lockedPlayerYaw;
        player.rotationYawHead = lockedPlayerYaw;
        player.prevRenderYawOffset = lockedPlayerYaw;
        player.prevRotationYawHead = lockedPlayerYaw;
    }

    private void renderCustomNametag(
            EntityPlayer player,
            String name,
            double x,
            double y,
            double z,
            boolean showBadge) {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.getRenderManager() != null && mc.getRenderManager().livingPlayer != null && player != mc.getRenderManager().livingPlayer) {
            double distanceSq = player.getDistanceSqToEntity(mc.getRenderManager().livingPlayer);
            if (distanceSq > 4096.0D) {
                return;
            }
        }

        float yaw;
        float pitch;
        if (perspectiveActive) {
            yaw = cameraYaw - 180.0F;
            pitch = cameraPitch;
        } else if (mc.getRenderManager() != null) {
            yaw = mc.getRenderManager().playerViewY;
            pitch = mc.getRenderManager().playerViewX;
        } else {
            yaw = player.rotationYaw;
            pitch = player.rotationPitch;
        }

        float heightOffset = player.height + 0.5F - (player.isSneaking() ? 0.25F : 0.0F);
        float fontScale = 0.016666668F * 1.6F;
        boolean sneaking = player.isSneaking();
        boolean self = player == mc.thePlayer;
        int bgTop = PrivetBadgeRenderer.backgroundTop(showBadge);
        int bgBottom = PrivetBadgeRenderer.backgroundBottom(showBadge);
        int halfWidth = PrivetBadgeRenderer.backgroundHalfWidth(mc.fontRendererObj, name, showBadge);
        int textX = PrivetBadgeRenderer.resolveTextX(mc.fontRendererObj, name, showBadge);

        // GlStateManager caches GL_FOG. When another mod (OptiFine, shaders) toggles
        // fog through raw GL11 the cache desyncs and disableFog() silently no-ops,
        // leaving the tag tinted by water/lava fog colour. Read and drive the real
        // GL state instead, then restore exactly what was there.
        boolean fogWasEnabled = GL11.glIsEnabled(GL11.GL_FOG);

        GlStateManager.pushMatrix();
        try {
            GlStateManager.translate((float) x, (float) y + heightOffset, (float) z);
            GL11.glNormal3f(0.0F, 1.0F, 0.0F);
            GlStateManager.rotate(-yaw, 0.0F, 1.0F, 0.0F);
            GlStateManager.rotate(pitch, 1.0F, 0.0F, 0.0F);
            GlStateManager.scale(-fontScale, -fontScale, fontScale);

            GlStateManager.disableLighting();
            GL11.glDisable(GL11.GL_FOG);
            GlStateManager.enableBlend();
            GlStateManager.tryBlendFuncSeparate(770, 771, 1, 0);
            GlStateManager.enableAlpha();
            GlStateManager.alphaFunc(GL11.GL_GREATER, 0.1F);
            GlStateManager.color(1.0F, 1.0F, 1.0F, 1.0F);

            Tessellator tessellator = Tessellator.getInstance();
            WorldRenderer worldrenderer = tessellator.getWorldRenderer();

            // See-through plate only (no text). Drawing translucent + opaque text at the
            // same pixels z-fights under OptiFine / sky views.
            if (!sneaking && !self) {
                GlStateManager.depthMask(false);
                GlStateManager.disableDepth();
                drawNametagPlate(worldrenderer, halfWidth, bgTop, bgBottom, 0.25F);
            }

            // Single solid pass: plate + badge + opaque name.
            //
            // The local F5 tag must ignore depth so clouds/sky never punch holes in
            // the nick, but it still has to STAMP depth: water is translucent and
            // renders after entities, so without a depth value of our own the water
            // pass blends straight over the tag and tints it blue. Disabling
            // GL_DEPTH_TEST also disables depth writes regardless of depthMask, so
            // keep the test enabled and make it always pass instead.
            if (self || sneaking) {
                GlStateManager.enableDepth();
                GlStateManager.depthFunc(GL11.GL_ALWAYS);
                GlStateManager.depthMask(true);
            } else {
                GlStateManager.enableDepth();
                GlStateManager.depthMask(true);
                GlStateManager.depthFunc(GL11.GL_LEQUAL);
            }

            // Vanilla / Lunar nametag plate alpha (64/255).
            drawNametagPlate(worldrenderer, halfWidth, bgTop, bgBottom, 0.25F);

            // Badge: blended antialiased mark, sized to stay inside the plate.
            GlStateManager.enableTexture2D();
            GlStateManager.color(1.0F, 1.0F, 1.0F, 1.0F);
            PrivetBadgeRenderer.drawNametagBadge(mc.fontRendererObj, name, showBadge);

            // Name last so FontRenderer rebinds its atlas after the badge texture.
            GlStateManager.enableTexture2D();
            GlStateManager.enableAlpha();
            GlStateManager.alphaFunc(GL11.GL_GREATER, 0.1F);
            GlStateManager.enableBlend();
            GlStateManager.tryBlendFuncSeparate(770, 771, 1, 0);
            GlStateManager.color(1.0F, 1.0F, 1.0F, 1.0F);
            mc.fontRendererObj.drawString(name, textX, 0, -1);
        } finally {
            GlStateManager.enableTexture2D();
            GlStateManager.enableDepth();
            GlStateManager.depthMask(true);
            GlStateManager.depthFunc(GL11.GL_LEQUAL);
            if (fogWasEnabled) {
                GL11.glEnable(GL11.GL_FOG);
            } else {
                GL11.glDisable(GL11.GL_FOG);
            }
            GlStateManager.enableLighting();
            GlStateManager.disableBlend();
            GlStateManager.enableAlpha();
            GlStateManager.alphaFunc(GL11.GL_GREATER, 0.1F);
            GlStateManager.color(1.0F, 1.0F, 1.0F, 1.0F);
            GlStateManager.popMatrix();
        }
    }

    private static void drawNametagPlate(
            WorldRenderer worldrenderer,
            int halfWidth,
            int bgTop,
            int bgBottom,
            float alpha) {
        GlStateManager.disableTexture2D();
        worldrenderer.begin(7, DefaultVertexFormats.POSITION_COLOR);
        worldrenderer.pos(-halfWidth - 1, bgTop, 0.0D).color(0.0F, 0.0F, 0.0F, alpha).endVertex();
        worldrenderer.pos(-halfWidth - 1, bgBottom, 0.0D).color(0.0F, 0.0F, 0.0F, alpha).endVertex();
        worldrenderer.pos(halfWidth + 1, bgBottom, 0.0D).color(0.0F, 0.0F, 0.0F, alpha).endVertex();
        worldrenderer.pos(halfWidth + 1, bgTop, 0.0D).color(0.0F, 0.0F, 0.0F, alpha).endVertex();
        Tessellator.getInstance().draw();
    }

    private void renderInformationalHud() {
        if (!modules.get(FpsDisplayModule.ID).isEnabled()) {
            return;
        }
        Minecraft minecraft = Minecraft.getMinecraft();
        if (minecraft.fontRendererObj == null || minecraft.gameSettings.showDebugInfo) {
            return;
        }
        minecraft.fontRendererObj.drawStringWithShadow(
                FpsDisplayUtil.format(Minecraft.getDebugFPS()),
                config.getHudOffsetX(),
                config.getHudOffsetY(),
                0xFFFFFF);
    }

    private void maskTabListNames() {
        restoreTabListNames();
        Minecraft minecraft = Minecraft.getMinecraft();
        if (minecraft.getNetHandler() == null) {
            return;
        }
        List<NetworkPlayerInfo> players = new ArrayList<NetworkPlayerInfo>(
                minecraft.getNetHandler().getPlayerInfoMap());
        Collections.sort(players, new Comparator<NetworkPlayerInfo>() {
            @Override
            public int compare(NetworkPlayerInfo left, NetworkPlayerInfo right) {
                return left.getGameProfile().getId().compareTo(right.getGameProfile().getId());
            }
        });
        int index = 1;
        for (NetworkPlayerInfo player : players) {
            IChatComponent original = player.getDisplayName();
            tabDisplayNameSnapshot.put(player, original);
            String alias = StreamerModeUtil.playerAlias(index++);
            String formattedAlias = ScorePlayerTeam.formatPlayerName(player.getPlayerTeam(), alias);
            player.setDisplayName(new ChatComponentText(formattedAlias));
        }
    }

    private void restoreTabListNames() {
        for (Map.Entry<NetworkPlayerInfo, IChatComponent> entry : tabDisplayNameSnapshot.entrySet()) {
            entry.getKey().setDisplayName(entry.getValue());
        }
        tabDisplayNameSnapshot.clear();
    }

    private Collection<String> getKnownPlayerNames() {
        List<String> names = new ArrayList<String>();
        Minecraft minecraft = Minecraft.getMinecraft();
        if (minecraft.getNetHandler() != null) {
            for (NetworkPlayerInfo player : minecraft.getNetHandler().getPlayerInfoMap()) {
                if (player.getGameProfile() != null) {
                    names.add(player.getGameProfile().getName());
                }
            }
        }
        if (minecraft.thePlayer != null) {
            names.add(minecraft.thePlayer.getName());
        }
        return names;
    }

    private void refreshSession() {
        try {
            sessionObserver.refresh();
        } catch (IOException exception) {
            log.error("Could not update the local profile bridge", exception);
        }
    }

    public void ensureWindowTitle() {
        if (Display.isCreated()
                && !WindowTitleTransformer.PRIVATE_TITLE.equals(Display.getTitle())) {
            Display.setTitle(WindowTitleTransformer.PRIVATE_TITLE);
        }
    }

}

