# Modrinth Integration

Search calls the official Modrinth v2 API with facets for:

```text
project_type = mod
versions = 1.8.9
categories/loaders = forge
```

Query text is sent as entered after length/control-character validation. Username,
UUID, account/session data, active server, local paths, hardware data, and the
installed-mod list are not attached. Search history is not persisted by default.

Results are only summaries. Before installation the backend fetches the project
and all versions, then selects an exact compatible version and exact primary JAR.
Preference is release then beta. Alpha is blocked without explicit confirmation.
Source/dev/deobf/documentation artifacts are rejected.

Required dependency edges are resolved before download. The algorithm keeps a
visiting/visited set to detect cycles and records the final directed graph. A
missing or incompatible required dependency blocks the root install.

Only official `api.modrinth.com` and `cdn.modrinth.com` hosts are accepted. Every
redirect is checked. Provider SHA-512/SHA-1 are verified and a fresh local
SHA-512 is stored. The cache is bounded, versioned, TTL-based, and ignored if its
schema/content fails validation. Offline mode may show cache and installed mods
but never pretends that a new install completed.

Dynamic projects are labeled `FROM MODRINTH`. Only a specific project/version
combination listed with review evidence in the controlled manifest may display
`VERIFIED`. Unknown license metadata is shown as `LICENSE REVIEW`.

Tests use recorded, anonymized fixtures and a local HTTP server for paging,
filtering, dependency graphs, redirects, truncation, hash mismatch, rollback, and
offline behavior.
