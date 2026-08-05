package client.privateclient.modules.api;

import client.privateclient.events.CoreEventBus;
import java.util.Objects;

public final class ModuleContext {
    private final CoreEventBus eventBus;

    public ModuleContext(CoreEventBus eventBus) {
        this.eventBus = Objects.requireNonNull(eventBus, "eventBus");
    }

    public CoreEventBus getEventBus() {
        return eventBus;
    }
}
