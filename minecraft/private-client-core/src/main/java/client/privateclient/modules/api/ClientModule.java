package client.privateclient.modules.api;

import java.util.Map;
import java.util.Set;

public interface ClientModule {
    String getId();

    String getName();

    String getDescription();

    ModuleCategory getCategory();

    boolean isDefaultEnabled();

    boolean isEnabled();

    Set<String> getDependencies();

    Set<String> getConflicts();

    void validateConfiguration(Map<String, String> configuration);

    void enable() throws ModuleActivationException;

    void disable() throws ModuleActivationException;
}
