package client.privateclient.srv;

import client.privateclient.branding.SplashProgressTransformer;
import client.privateclient.branding.WindowTitleTransformer;
import java.util.Map;
import net.minecraftforge.fml.relauncher.IFMLLoadingPlugin;

@IFMLLoadingPlugin.MCVersion("1.8.9")
@IFMLLoadingPlugin.Name("PrivateClientSrvResolver")
@IFMLLoadingPlugin.TransformerExclusions({
        "client.privateclient.srv",
        "client.privateclient.branding"
})
public final class PrivateClientLoadingPlugin implements IFMLLoadingPlugin {
    @Override
    public String[] getASMTransformerClass() {
        return new String[] {
                WindowTitleTransformer.class.getName(),
                SplashProgressTransformer.class.getName(),
                ServerAddressTransformer.class.getName()
        };
    }

    @Override
    public String getModContainerClass() {
        return null;
    }

    @Override
    public String getSetupClass() {
        return null;
    }

    @Override
    public void injectData(Map<String, Object> data) {
    }

    @Override
    public String getAccessTransformerClass() {
        return null;
    }
}
