package client.privateclient.modules.api;

import java.util.Arrays;
import java.util.HashSet;
import java.util.Locale;
import java.util.Set;

public final class SafeModulePolicy {
    private static final Set<String> FORBIDDEN_TERMS = new HashSet<String>(Arrays.asList(
            "aim",
            "aura",
            "autoclick",
            "bypass",
            "esp",
            "fly",
            "killaura",
            "noclip",
            "reach",
            "scaffold",
            "silentaim",
            "triggerbot",
            "velocity"
    ));

    private SafeModulePolicy() {
    }

    public static void verify(ClientModule module) {
        if (module == null) {
            throw new IllegalArgumentException("Module is required");
        }
        if (module.getCategory() != ModuleCategory.INFORMATIONAL
                && module.getCategory() != ModuleCategory.COSMETIC
                && module.getCategory() != ModuleCategory.PERFORMANCE) {
            throw new IllegalArgumentException("Unsupported module category");
        }

        String normalized = (module.getId() + " " + module.getName())
                .toLowerCase(Locale.ROOT)
                .replaceAll("[^a-z0-9]", "");
        for (String term : FORBIDDEN_TERMS) {
            if (normalized.contains(term)) {
                throw new IllegalArgumentException("Forbidden module capability: " + term);
            }
        }
    }
}
