# Private Client Association API

Server-scoped presence for PrivetBadge (Lunar-style association).

Production: https://private-client-association.vercel.app

## Endpoints

- `POST /api/v1/presence` — heartbeat upsert + peer list for a `serverHash`
- `GET /api/health` — liveness

## Storage

Presence records are stored in **Vercel Blob** under
`presence/{serverHash}/{uuid}__{username}.json` and filtered by upload time
(TTL ~45s). Responses include both `peers` (UUID list) and `peerEntries`
(`{ uuid, username }`) so offline-mode clients can match by name.

## Deploy

```powershell
cd apps/association-api
pnpm exec vercel link --yes --scope risky5 --project private-client-association
pnpm exec vercel --prod --yes
```

Core clients call `https://private-client-association.vercel.app/api/v1/presence`.

## Privacy

Payload contains only `uuid`, `username`, SHA-256 `serverHash`, and `clientVersion`.
No Minecraft session tokens, passwords, or raw server hostnames are accepted or stored.
