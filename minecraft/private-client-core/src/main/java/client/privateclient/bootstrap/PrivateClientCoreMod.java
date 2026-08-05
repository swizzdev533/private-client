package client.privateclient.bootstrap;

import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.event.FMLLoadCompleteEvent;
import net.minecraftforge.fml.common.event.FMLPreInitializationEvent;

@Mod(
        modid = PrivateClientCoreMod.MOD_ID,
        name = PrivateClientCoreMod.NAME,
        version = PrivateClientCoreMod.VERSION,
        acceptedMinecraftVersions = "[1.8.9]",
        acceptableRemoteVersions = "*",
        clientSideOnly = true,
        useMetadata = true)
public final class PrivateClientCoreMod {
    public static final String MOD_ID = "privateclientcore";
    public static final String NAME = "Private Client Core";
    public static final String VERSION = "1.0.0";

    private ClientBootstrap bootstrap;

    @Mod.EventHandler
    public void preInitialize(FMLPreInitializationEvent event) {
        bootstrap = ClientBootstrap.start(event.getModLog());
    }

    @Mod.EventHandler
    public void loadComplete(FMLLoadCompleteEvent event) {
        if (bootstrap != null) {
            bootstrap.ready();
        }
    }
}
