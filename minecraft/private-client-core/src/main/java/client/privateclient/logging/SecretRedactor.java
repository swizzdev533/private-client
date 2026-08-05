package client.privateclient.logging;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class SecretRedactor {
    private static final String REDACTED = "[REDACTED]";

    private static final Pattern AUTHORIZATION = Pattern.compile(
            "(?i)(\\bauthorization\\s*[:=]\\s*(?:bearer\\s+)?)([^\\s,;]+)");
    private static final Pattern BEARER = Pattern.compile(
            "(?i)(\\bbearer\\s+)([A-Za-z0-9._~+/=-]+)");
    private static final Pattern SENSITIVE_FIELD = Pattern.compile(
            "(?i)([\"']?(?:access[_-]?token|refresh[_-]?token|client[_-]?secret|"
                    + "session[_-]?(?:id|token)?|password|passwd|cookie|oauth[_-]?token)"
                    + "[\"']?\\s*[:=]\\s*[\"']?)([^\"'\\s,;&}]+)");
    private static final Pattern COOKIE_HEADER = Pattern.compile(
            "(?i)(\\b(?:set-cookie|cookie)\\s*:\\s*)([^\\r\\n]+)");

    public String redact(String input) {
        if (input == null) {
            return null;
        }
        String output = replaceValue(AUTHORIZATION, input);
        output = replaceValue(BEARER, output);
        output = replaceValue(SENSITIVE_FIELD, output);
        output = replaceValue(COOKIE_HEADER, output);
        return output;
    }

    private static String replaceValue(Pattern pattern, String input) {
        Matcher matcher = pattern.matcher(input);
        StringBuffer output = new StringBuffer();
        while (matcher.find()) {
            matcher.appendReplacement(
                    output,
                    Matcher.quoteReplacement(matcher.group(1) + REDACTED));
        }
        matcher.appendTail(output);
        return output.toString();
    }
}
