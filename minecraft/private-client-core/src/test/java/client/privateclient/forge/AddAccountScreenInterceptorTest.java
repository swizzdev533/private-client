package client.privateclient.forge;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

import client.privateclient.gui.GuiAddOfflineAccount;
import client.privateclient.logging.SafeLogger;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.gui.GuiScreen;
import org.apache.logging.log4j.LogManager;
import org.junit.Before;
import org.junit.Test;

public final class AddAccountScreenInterceptorTest {
    private final SafeLogger log = new SafeLogger(LogManager.getLogger(getClass()));

    @Before
    public void resetStorage() {
        FakeAltDatabase.reset();
        FakeConfig.saves = 0;
    }

    @Test
    public void replacesTheIasAddAccountScreen() {
        GuiScreen replacement =
                interceptor().replace(new FakeAddAccountScreen(new GuiScreen() { }), log);

        assertTrue(replacement instanceof GuiAddOfflineAccount);
    }

    @Test
    public void ignoresUnrelatedScreens() {
        assertNull(interceptor().replace(new GuiScreen() { }, log));
        assertNull(interceptor().replace(null, log));
    }

    @Test
    public void storesOfflineNameWithoutAPassword() throws Exception {
        assertTrue(repository().add("Swizz5"));

        assertEquals(1, FakeAltDatabase.accounts.size());
        FakeAccountData stored = (FakeAccountData) FakeAltDatabase.accounts.get(0);
        assertEquals("enc:Swizz5", stored.user);
        assertEquals("enc:", stored.pass);
        assertEquals("Swizz5", stored.alias);
        assertEquals(1, FakeConfig.saves);
    }

    @Test
    public void rejectsDuplicateNames() throws Exception {
        assertTrue(repository().add("Swizz5"));
        assertFalse(repository().add("swizz5"));
        assertEquals(1, FakeAltDatabase.accounts.size());
    }

    @Test
    public void rejectsNamesThatAreNotMinecraftNames() {
        assertFalse(OfflineAccountRepository.isValidName(null));
        assertFalse(OfflineAccountRepository.isValidName("ab"));
        assertFalse(OfflineAccountRepository.isValidName("seventeen_chars_x"));
        assertFalse(OfflineAccountRepository.isValidName("bad name"));
        assertFalse(OfflineAccountRepository.isValidName("mail@example.com"));
        assertTrue(OfflineAccountRepository.isValidName("Swizz5"));
        assertTrue(OfflineAccountRepository.isValidName("kocieruchy533"));
    }

    private AddAccountScreenInterceptor interceptor() {
        return new AddAccountScreenInterceptor(FakeAddAccountScreen.class.getName(), repository());
    }

    private static OfflineAccountRepository repository() {
        return new OfflineAccountRepository(
                FakeAltDatabase.class.getName(),
                FakeAccountData.class.getName(),
                FakeEncryptionTools.class.getName(),
                FakeConfig.class.getName());
    }

    public static class FakeAddAccountScreen extends GuiScreen {
        public final GuiScreen prev;

        public FakeAddAccountScreen(GuiScreen prev) {
            this.prev = prev;
        }
    }

    public static final class FakeAltDatabase {
        static final List<Object> accounts = new ArrayList<Object>();
        private static final FakeAltDatabase INSTANCE = new FakeAltDatabase();

        static void reset() {
            accounts.clear();
        }

        public static FakeAltDatabase getInstance() {
            return INSTANCE;
        }

        public List<Object> getAlts() {
            return accounts;
        }
    }

    public static final class FakeAccountData {
        public final String user;
        public final String pass;
        public final String alias;

        public FakeAccountData(String user, String pass, String alias) {
            this.user = FakeEncryptionTools.encode(user);
            this.pass = FakeEncryptionTools.encode(pass);
            this.alias = alias;
        }
    }

    public static final class FakeEncryptionTools {
        public static String encode(String value) {
            return "enc:" + value;
        }

        public static String decode(String value) {
            return value != null && value.startsWith("enc:") ? value.substring(4) : value;
        }
    }

    public static final class FakeConfig {
        static int saves;

        public static void save() {
            saves++;
        }
    }
}
