package client.privateclient.srv;

import java.net.IDN;
import java.util.ArrayList;
import java.util.Hashtable;
import java.util.List;
import java.util.concurrent.ThreadLocalRandom;
import javax.naming.NamingEnumeration;
import javax.naming.directory.Attribute;
import javax.naming.directory.Attributes;
import javax.naming.directory.DirContext;
import javax.naming.directory.InitialDirContext;

public final class SrvResolver {
    static final int DEFAULT_PORT = 25565;

    interface Lookup {
        List<String> lookup(String name) throws Exception;
    }

    private static final Lookup DNS_LOOKUP = new JndiLookup();

    private SrvResolver() {
    }

    public static ResolvedServerAddress resolve(String input) {
        return resolve(input, DNS_LOOKUP);
    }

    static ResolvedServerAddress resolve(String input, Lookup lookup) {
        ParsedAddress parsed = parse(input);
        if (parsed.explicitPort || !isSrvDomain(parsed.host)) {
            return new ResolvedServerAddress(parsed.host, parsed.port);
        }

        try {
            SrvRecord record = select(parseRecords(lookup.lookup("_minecraft._tcp." + parsed.host)));
            if (record != null) {
                return new ResolvedServerAddress(record.target, record.port);
            }
        } catch (Exception ignored) {
            // DNS failures must never prevent the normal direct connection attempt.
        }
        return new ResolvedServerAddress(parsed.host, parsed.port);
    }

    private static ParsedAddress parse(String input) {
        String value = input == null ? "" : input.trim();
        if (value.startsWith("[") && value.contains("]")) {
            int end = value.indexOf(']');
            String host = value.substring(1, end);
            if (end + 1 < value.length() && value.charAt(end + 1) == ':') {
                Integer port = parsePort(value.substring(end + 2));
                if (port != null) {
                    return new ParsedAddress(host, port, true);
                }
            }
            return new ParsedAddress(host, DEFAULT_PORT, false);
        }

        int firstColon = value.indexOf(':');
        int lastColon = value.lastIndexOf(':');
        if (firstColon > 0 && firstColon == lastColon) {
            Integer port = parsePort(value.substring(firstColon + 1));
            if (port != null) {
                return new ParsedAddress(value.substring(0, firstColon), port, true);
            }
        }
        return new ParsedAddress(value, DEFAULT_PORT, false);
    }

    private static Integer parsePort(String value) {
        try {
            int port = Integer.parseInt(value);
            return port >= 1 && port <= 65535 ? port : null;
        } catch (NumberFormatException ignored) {
            return null;
        }
    }

    private static boolean isSrvDomain(String host) {
        if (host.isEmpty() || host.indexOf(':') >= 0 || host.equalsIgnoreCase("localhost")) {
            return false;
        }
        try {
            String ascii = IDN.toASCII(host);
            if (ascii.length() > 253 || ascii.indexOf('.') < 1) {
                return false;
            }
            for (String label : ascii.split("\\.")) {
                if (label.isEmpty() || label.length() > 63 || label.startsWith("-") || label.endsWith("-")
                        || !label.matches("[A-Za-z0-9-]+")) {
                    return false;
                }
            }
            return true;
        } catch (IllegalArgumentException ignored) {
            return false;
        }
    }

    private static List<SrvRecord> parseRecords(List<String> values) {
        List<SrvRecord> records = new ArrayList<SrvRecord>();
        for (String value : values) {
            String[] fields = value.trim().split("\\s+");
            if (fields.length != 4) {
                continue;
            }
            try {
                int priority = Integer.parseInt(fields[0]);
                int weight = Integer.parseInt(fields[1]);
                int port = Integer.parseInt(fields[2]);
                String target = fields[3].endsWith(".")
                        ? fields[3].substring(0, fields[3].length() - 1) : fields[3];
                if (priority >= 0 && weight >= 0 && port >= 1 && port <= 65535 && isSrvDomain(target)) {
                    records.add(new SrvRecord(priority, weight, port, target));
                }
            } catch (NumberFormatException ignored) {
                // Ignore malformed records and keep checking the remaining answers.
            }
        }
        return records;
    }

    private static SrvRecord select(List<SrvRecord> records) {
        int minimumPriority = Integer.MAX_VALUE;
        for (SrvRecord record : records) {
            minimumPriority = Math.min(minimumPriority, record.priority);
        }
        List<SrvRecord> eligible = new ArrayList<SrvRecord>();
        int totalWeight = 0;
        for (SrvRecord record : records) {
            if (record.priority == minimumPriority) {
                eligible.add(record);
                totalWeight += record.weight;
            }
        }
        if (eligible.isEmpty()) {
            return null;
        }
        if (totalWeight <= 0) {
            return eligible.get(0);
        }
        int choice = ThreadLocalRandom.current().nextInt(totalWeight) + 1;
        for (SrvRecord record : eligible) {
            choice -= record.weight;
            if (choice <= 0) {
                return record;
            }
        }
        return eligible.get(eligible.size() - 1);
    }

    private static final class JndiLookup implements Lookup {
        @Override
        public List<String> lookup(String name) throws Exception {
            Hashtable<String, String> environment = new Hashtable<String, String>();
            environment.put("java.naming.factory.initial", "com.sun.jndi.dns.DnsContextFactory");
            environment.put("com.sun.jndi.dns.timeout.initial", "1500");
            environment.put("com.sun.jndi.dns.timeout.retries", "1");
            DirContext context = new InitialDirContext(environment);
            try {
                Attributes attributes = context.getAttributes(name, new String[] {"SRV"});
                Attribute attribute = attributes.get("SRV");
                List<String> results = new ArrayList<String>();
                if (attribute != null) {
                    NamingEnumeration<?> values = attribute.getAll();
                    try {
                        while (values.hasMore()) {
                            results.add(String.valueOf(values.next()));
                        }
                    } finally {
                        values.close();
                    }
                }
                return results;
            } finally {
                context.close();
            }
        }
    }

    private static final class ParsedAddress {
        private final String host;
        private final int port;
        private final boolean explicitPort;

        private ParsedAddress(String host, int port, boolean explicitPort) {
            this.host = host;
            this.port = port;
            this.explicitPort = explicitPort;
        }
    }

    private static final class SrvRecord {
        private final int priority;
        private final int weight;
        private final int port;
        private final String target;

        private SrvRecord(int priority, int weight, int port, String target) {
            this.priority = priority;
            this.weight = weight;
            this.port = port;
            this.target = target;
        }
    }
}
