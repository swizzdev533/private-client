package client.privateclient.modules.api;

import client.privateclient.events.CoreEvent;
import client.privateclient.events.CoreEventBus;
import client.privateclient.events.CoreEventType;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;

public final class ModuleRegistry {
    private final Map<String, ClientModule> modules = new LinkedHashMap<String, ClientModule>();
    private final CoreEventBus eventBus;

    public ModuleRegistry(CoreEventBus eventBus) {
        this.eventBus = Objects.requireNonNull(eventBus, "eventBus");
    }

    public synchronized void register(ClientModule module) {
        Objects.requireNonNull(module, "module");
        SafeModulePolicy.verify(module);
        if (modules.containsKey(module.getId())) {
            throw new IllegalArgumentException("Duplicate module id: " + module.getId());
        }
        modules.put(module.getId(), module);
    }

    public synchronized ClientModule get(String id) {
        ClientModule module = modules.get(id);
        if (module == null) {
            throw new IllegalArgumentException("Unknown module: " + id);
        }
        return module;
    }

    public synchronized List<ClientModule> all() {
        return Collections.unmodifiableList(new ArrayList<ClientModule>(modules.values()));
    }

    public synchronized void enable(String id) throws ModuleActivationException {
        List<ClientModule> enabledThisOperation = new ArrayList<ClientModule>();
        try {
            enableRecursively(id, new LinkedHashSet<String>(), enabledThisOperation);
        } catch (ModuleActivationException exception) {
            rollback(enabledThisOperation);
            throw exception;
        } catch (RuntimeException exception) {
            rollback(enabledThisOperation);
            throw new ModuleActivationException("Could not enable module " + id, exception);
        }
    }

    public synchronized void disable(String id) throws ModuleActivationException {
        ClientModule module = get(id);
        for (ClientModule candidate : modules.values()) {
            if (candidate.isEnabled() && candidate.getDependencies().contains(id)) {
                throw new ModuleActivationException(
                        "Module " + id + " is required by enabled module " + candidate.getId());
            }
        }
        if (module.isEnabled()) {
            module.disable();
            eventBus.publish(CoreEvent.withCode(CoreEventType.MODULE_DISABLED, module.getId()));
        }
    }

    public synchronized Map<String, Boolean> snapshotStates() {
        Map<String, Boolean> result = new LinkedHashMap<String, Boolean>();
        for (ClientModule module : modules.values()) {
            result.put(module.getId(), module.isEnabled());
        }
        return Collections.unmodifiableMap(result);
    }

    public synchronized void disableAll() {
        List<ClientModule> reverse = new ArrayList<ClientModule>(modules.values());
        Collections.reverse(reverse);
        for (ClientModule module : reverse) {
            if (!module.isEnabled()) {
                continue;
            }
            try {
                module.disable();
                eventBus.publish(CoreEvent.withCode(CoreEventType.MODULE_DISABLED, module.getId()));
            } catch (ModuleActivationException ignored) {
                // Shutdown must continue so remaining modules can release their resources.
            }
        }
    }

    private void enableRecursively(
            String id,
            Set<String> activationPath,
            List<ClientModule> enabledThisOperation) throws ModuleActivationException {
        ClientModule module = get(id);
        if (module.isEnabled()) {
            return;
        }
        if (!activationPath.add(id)) {
            throw new ModuleActivationException("Circular module dependency at " + id);
        }

        for (String conflictId : module.getConflicts()) {
            ClientModule conflict = modules.get(conflictId);
            if (conflict != null && conflict.isEnabled()) {
                throw new ModuleActivationException(
                        "Module " + id + " conflicts with " + conflictId);
            }
        }
        for (ClientModule candidate : modules.values()) {
            if (candidate.isEnabled() && candidate.getConflicts().contains(id)) {
                throw new ModuleActivationException(
                        "Module " + id + " conflicts with " + candidate.getId());
            }
        }
        for (String dependency : module.getDependencies()) {
            enableRecursively(dependency, activationPath, enabledThisOperation);
        }

        activationPath.remove(id);
        module.enable();
        enabledThisOperation.add(module);
        eventBus.publish(CoreEvent.withCode(CoreEventType.MODULE_ENABLED, module.getId()));
    }

    private void rollback(List<ClientModule> enabledThisOperation) {
        Collections.reverse(enabledThisOperation);
        for (ClientModule module : enabledThisOperation) {
            try {
                module.disable();
                eventBus.publish(CoreEvent.withCode(CoreEventType.MODULE_DISABLED, module.getId()));
            } catch (ModuleActivationException ignored) {
                // The original activation error remains the primary failure.
            }
        }
    }
}
