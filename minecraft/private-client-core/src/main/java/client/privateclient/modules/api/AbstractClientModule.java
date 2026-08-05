package client.privateclient.modules.api;

import java.util.Collections;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.concurrent.atomic.AtomicBoolean;

public abstract class AbstractClientModule implements ClientModule {
    private final String id;
    private final String name;
    private final String description;
    private final ModuleCategory category;
    private final boolean defaultEnabled;
    private final Set<String> dependencies;
    private final Set<String> conflicts;
    private final ModuleContext context;
    private final AtomicBoolean enabled = new AtomicBoolean(false);
    private final Object lifecycleLock = new Object();

    protected AbstractClientModule(
            String id,
            String name,
            String description,
            ModuleCategory category,
            boolean defaultEnabled,
            Set<String> dependencies,
            Set<String> conflicts,
            ModuleContext context) {
        this.id = validateId(id);
        this.name = requireText(name, "name");
        this.description = requireText(description, "description");
        this.category = Objects.requireNonNull(category, "category");
        this.defaultEnabled = defaultEnabled;
        this.dependencies = immutableIds(dependencies);
        this.conflicts = immutableIds(conflicts);
        this.context = Objects.requireNonNull(context, "context");
        if (this.dependencies.contains(this.id) || this.conflicts.contains(this.id)) {
            throw new IllegalArgumentException("A module cannot depend on or conflict with itself");
        }
    }

    @Override
    public final String getId() {
        return id;
    }

    @Override
    public final String getName() {
        return name;
    }

    @Override
    public final String getDescription() {
        return description;
    }

    @Override
    public final ModuleCategory getCategory() {
        return category;
    }

    @Override
    public final boolean isDefaultEnabled() {
        return defaultEnabled;
    }

    @Override
    public final boolean isEnabled() {
        return enabled.get();
    }

    @Override
    public final Set<String> getDependencies() {
        return dependencies;
    }

    @Override
    public final Set<String> getConflicts() {
        return conflicts;
    }

    @Override
    public void validateConfiguration(Map<String, String> configuration) {
        Objects.requireNonNull(configuration, "configuration");
    }

    @Override
    public final void enable() throws ModuleActivationException {
        synchronized (lifecycleLock) {
            if (enabled.get()) {
                return;
            }
            try {
                onEnable(context);
                enabled.set(true);
            } catch (RuntimeException exception) {
                throw new ModuleActivationException("Could not enable module " + id, exception);
            }
        }
    }

    @Override
    public final void disable() throws ModuleActivationException {
        synchronized (lifecycleLock) {
            if (!enabled.get()) {
                return;
            }
            try {
                onDisable(context);
                enabled.set(false);
            } catch (RuntimeException exception) {
                throw new ModuleActivationException("Could not disable module " + id, exception);
            }
        }
    }

    protected void onEnable(ModuleContext context) {
    }

    protected void onDisable(ModuleContext context) {
    }

    private static String validateId(String value) {
        String id = requireText(value, "id");
        if (!id.matches("[a-z][a-z0-9-]{1,47}")) {
            throw new IllegalArgumentException("Invalid module id: " + id);
        }
        return id;
    }

    private static String requireText(String value, String field) {
        if (value == null || value.trim().isEmpty()) {
            throw new IllegalArgumentException(field + " is required");
        }
        return value.trim();
    }

    private static Set<String> immutableIds(Set<String> values) {
        if (values == null || values.isEmpty()) {
            return Collections.emptySet();
        }
        Set<String> result = new LinkedHashSet<String>();
        for (String value : values) {
            result.add(validateId(value));
        }
        return Collections.unmodifiableSet(result);
    }
}
