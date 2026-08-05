package client.privateclient.bootstrap;

import client.privateclient.association.AssociationService;
import client.privateclient.auth.SessionObserver;
import client.privateclient.auth.SessionPolicy;
import client.privateclient.config.ConfigLoadResult;
import client.privateclient.config.ConfigStore;
import client.privateclient.config.CoreConfig;
import client.privateclient.discord.DiscordPresenceService;
import client.privateclient.events.CoreEvent;
import client.privateclient.events.CoreEventBus;
import client.privateclient.events.CoreEventType;
import client.privateclient.forge.ForgeClientHooks;
import client.privateclient.forge.ForgeSessionProvider;
import client.privateclient.logging.SafeLogger;
import client.privateclient.modules.api.ModuleActivationException;
import client.privateclient.modules.api.ModuleContext;
import client.privateclient.modules.api.ModuleRegistry;
import client.privateclient.modules.impl.ItemPhysicsModule;
import client.privateclient.modules.impl.FpsDisplayModule;
import client.privateclient.modules.impl.NametagsModule;
import client.privateclient.modules.impl.PerspectiveModule;
import client.privateclient.modules.impl.PrivateOptimizationModule;
import client.privateclient.modules.impl.ScoreboardModule;
import client.privateclient.modules.impl.ToggleSprintModule;
import client.privateclient.profile.ProfileBridge;
import client.privateclient.security.CorePaths;
import java.io.IOException;
import java.util.Map;
import java.util.concurrent.atomic.AtomicBoolean;
import org.apache.logging.log4j.Logger;

public final class ClientBootstrap {
    private final SafeLogger log;
    private final CoreEventBus eventBus;
    private final ConfigStore configStore;
    private final ModuleRegistry modules;
    private final ForgeClientHooks hooks;
    private final DiscordPresenceService discordPresence;
    private final AssociationService associationService;
    private final AtomicBoolean stopped = new AtomicBoolean(false);
    private volatile CoreConfig config;

    private ClientBootstrap(
            SafeLogger log,
            CoreEventBus eventBus,
            ConfigStore configStore,
            CoreConfig config,
            ModuleRegistry modules,
            ForgeClientHooks hooks,
            DiscordPresenceService discordPresence,
            AssociationService associationService) {
        this.log = log;
        this.eventBus = eventBus;
        this.configStore = configStore;
        this.config = config;
        this.modules = modules;
        this.hooks = hooks;
        this.discordPresence = discordPresence;
        this.associationService = associationService;
    }

    public static ClientBootstrap start(Logger forgeLogger) {
        final SafeLogger safeLog = new SafeLogger(forgeLogger);
        final CoreEventBus eventBus = new CoreEventBus((event, failure) ->
                safeLog.error("A Core event listener failed for " + event.getType().name(), failure));
        eventBus.publish(CoreEvent.of(CoreEventType.CLIENT_START));

        CorePaths paths = CorePaths.discover();
        ConfigStore configStore = new ConfigStore(paths.getConfigFile());
        CoreConfig config = loadConfig(configStore, safeLog);

        ModuleContext moduleContext = new ModuleContext(eventBus);
        ModuleRegistry modules = new ModuleRegistry(eventBus);
        modules.register(new ToggleSprintModule(moduleContext));
        modules.register(new PerspectiveModule(moduleContext));
        modules.register(new NametagsModule(moduleContext));
        modules.register(new PrivateOptimizationModule(moduleContext));
        modules.register(new ItemPhysicsModule(moduleContext));
        modules.register(new ScoreboardModule(moduleContext));
        modules.register(new FpsDisplayModule(moduleContext));
        enableConfiguredModules(modules, config.getModules(), safeLog);

        ProfileBridge profileBridge = new ProfileBridge(
                paths.getDataRoot(),
                paths.getProfileFile());
        SessionPolicy sessionPolicy = new SessionPolicy();
        SessionObserver sessionObserver = new SessionObserver(
                new ForgeSessionProvider(),
                sessionPolicy,
                profileBridge,
                eventBus,
                config.isProfileBridgeEnabled());
        DiscordPresenceService discordPresence = new DiscordPresenceService(safeLog);
        AssociationService associationService = new AssociationService(sessionObserver, safeLog);
        ForgeClientHooks hooks = new ForgeClientHooks(
                sessionObserver,
                modules,
                eventBus,
                configStore,
                config,
                discordPresence,
                associationService,
                safeLog);
        hooks.register();
        discordPresence.start(config.isDiscordPresenceEnabled());

        ClientBootstrap bootstrap = new ClientBootstrap(
                safeLog,
                eventBus,
                configStore,
                config,
                modules,
                hooks,
                discordPresence,
                associationService);
        bootstrap.installShutdownHook();
        safeLog.info("Private Client Core initialized");
        return bootstrap;
    }

    public synchronized void ready() {
        hooks.ensureWindowTitle();
        eventBus.publish(CoreEvent.of(CoreEventType.CLIENT_READY));
        log.info("Private Client Core ready");
    }

    public synchronized void stop() {
        if (!stopped.compareAndSet(false, true)) {
            return;
        }
        hooks.unregister();
        associationService.shutdown();
        discordPresence.stop();
        modules.disableAll();
        eventBus.publish(CoreEvent.of(CoreEventType.CLIENT_SHUTDOWN));
        log.info("Private Client Core stopped");
    }

    private void installShutdownHook() {
        Runtime.getRuntime().addShutdownHook(new Thread(new Runnable() {
            @Override
            public void run() {
                stop();
            }
        }, "private-client-shutdown"));
    }

    private static CoreConfig loadConfig(ConfigStore store, SafeLogger log) {
        try {
            ConfigLoadResult result = store.load();
            if (result.getStatus() == ConfigLoadResult.Status.MIGRATED) {
                log.info("Configuration migrated to version " + CoreConfig.CURRENT_SCHEMA_VERSION);
            } else if (result.getStatus() == ConfigLoadResult.Status.RECOVERED_CORRUPT) {
                log.warn("Corrupt configuration encountered and quarantined; defaults restored");
            }
            return result.getConfig();
        } catch (IOException exception) {
            log.error("Could not load configuration, using defaults", exception);
            return CoreConfig.defaults();
        }
    }

    private static void enableConfiguredModules(
            ModuleRegistry registry,
            Map<String, Boolean> states,
            SafeLogger log) {
        for (Map.Entry<String, Boolean> entry : states.entrySet()) {
            if (!entry.getValue()) {
                continue;
            }
            try {
                registry.enable(entry.getKey());
            } catch (ModuleActivationException exception) {
                log.warn("Failed to activate module " + entry.getKey() + " from configuration: " + exception.getMessage());
            }
        }
    }
}
