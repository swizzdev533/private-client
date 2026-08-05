package client.privateclient.forge;

import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Stores offline account names in the required, pinned IAS mod without bundling its classes.
 *
 * <p>Only the username is ever written. Private Client never asks for, keeps, or forwards a
 * Mojang password, so the password slot of the IAS account record is always empty.
 */
public final class OfflineAccountRepository {
    private static final String DATABASE_CLASS =
            "com.github.mrebhan.ingameaccountswitcher.tools.alt.AltDatabase";
    private static final String ACCOUNT_CLASS = "the_fireplace.ias.account.ExtendedAccountData";
    private static final String ENCRYPTION_CLASS = "the_fireplace.iasencrypt.EncryptionTools";
    private static final String CONFIG_CLASS =
            "com.github.mrebhan.ingameaccountswitcher.tools.Config";

    private static final int MINIMUM_NAME_LENGTH = 3;
    private static final int MAXIMUM_NAME_LENGTH = 16;

    private final String databaseClassName;
    private final String accountClassName;
    private final String encryptionClassName;
    private final String configClassName;

    public OfflineAccountRepository() {
        this(DATABASE_CLASS, ACCOUNT_CLASS, ENCRYPTION_CLASS, CONFIG_CLASS);
    }

    OfflineAccountRepository(
            String databaseClassName,
            String accountClassName,
            String encryptionClassName,
            String configClassName) {
        this.databaseClassName = databaseClassName;
        this.accountClassName = accountClassName;
        this.encryptionClassName = encryptionClassName;
        this.configClassName = configClassName;
    }

    /** An offline name is the only accepted input, so it must look like a Minecraft name. */
    public static boolean isValidName(String name) {
        if (name == null) {
            return false;
        }
        String trimmed = name.trim();
        if (trimmed.length() < MINIMUM_NAME_LENGTH || trimmed.length() > MAXIMUM_NAME_LENGTH) {
            return false;
        }
        for (int index = 0; index < trimmed.length(); index++) {
            char character = trimmed.charAt(index);
            boolean allowed = (character >= 'a' && character <= 'z')
                    || (character >= 'A' && character <= 'Z')
                    || (character >= '0' && character <= '9')
                    || character == '_';
            if (!allowed) {
                return false;
            }
        }
        return true;
    }

    /** Existing account names, decoded through the IAS storage format. */
    public List<String> listNames() throws ReflectiveOperationException {
        Class<?> encryptionClass = Class.forName(encryptionClassName);
        Method decode = encryptionClass.getMethod("decode", String.class);
        Field userField = accountUserField();
        List<String> names = new ArrayList<String>();
        for (Object account : accounts()) {
            Object encoded = userField.get(account);
            Object decoded = decode.invoke(null, encoded);
            if (decoded instanceof String) {
                names.add((String) decoded);
            }
        }
        return Collections.unmodifiableList(names);
    }

    public boolean containsName(String name) throws ReflectiveOperationException {
        String trimmed = name == null ? "" : name.trim();
        for (String existing : listNames()) {
            if (existing.equalsIgnoreCase(trimmed)) {
                return true;
            }
        }
        return false;
    }

    /**
     * Appends an offline account and persists the IAS config.
     *
     * @return {@code true} when the account was added, {@code false} when it already existed.
     */
    @SuppressWarnings("unchecked")
    public boolean add(String name) throws ReflectiveOperationException {
        String trimmed = name == null ? "" : name.trim();
        if (!isValidName(trimmed)) {
            throw new IllegalArgumentException("Offline name is not a valid Minecraft name");
        }
        if (containsName(trimmed)) {
            return false;
        }
        Class<?> accountClass = Class.forName(accountClassName);
        Constructor<?> constructor =
                accountClass.getConstructor(String.class, String.class, String.class);
        Object account = constructor.newInstance(trimmed, "", trimmed);
        ((List<Object>) accounts()).add(account);
        save();
        return true;
    }

    private void save() throws ReflectiveOperationException {
        Class<?> configClass = Class.forName(configClassName);
        configClass.getMethod("save").invoke(null);
    }

    private List<?> accounts() throws ReflectiveOperationException {
        Class<?> databaseClass = Class.forName(databaseClassName);
        Object database = databaseClass.getMethod("getInstance").invoke(null);
        Object alts = databaseClass.getMethod("getAlts").invoke(database);
        if (!(alts instanceof List)) {
            throw new IllegalStateException("IAS account storage is not a list");
        }
        return (List<?>) alts;
    }

    private Field accountUserField() throws ReflectiveOperationException {
        Class<?> accountClass = Class.forName(accountClassName);
        Class<?> current = accountClass;
        while (current != null) {
            try {
                Field field = current.getDeclaredField("user");
                field.setAccessible(true);
                return field;
            } catch (NoSuchFieldException ignored) {
                current = current.getSuperclass();
            }
        }
        throw new NoSuchFieldException("IAS account record has no user field");
    }
}
