# PrivetBadge association

Lunar-style launcher logos next to nicknames for authenticated Private Client
users on the same multiplayer server.

## Runtime

1. Core detects a remote join (`ClientConnectedToServerEvent`).
2. It hashes `lowercase(host) + ":" + port` with SHA-256.
3. Every 15s it POSTs `{ schemaVersion, uuid, username, serverHash, clientVersion }`
   to `https://private-client-association.vercel.app/api/v1/presence`.
4. The API upserts a Vercel Blob object and returns fresh peer UUIDs (TTL 45s).
5. Nametag and TAB render `privet_badge.png` for self and peers in the cache.

Failures are soft: gameplay continues; peer badges may disappear after the
60s local soft-retention window.

## Deploy

See [`apps/association-api/README.md`](../apps/association-api/README.md).
Point DNS for `association.privateclient.app` at the Vercel deployment and
apply `supabase/migrations/001_presence.sql`.
