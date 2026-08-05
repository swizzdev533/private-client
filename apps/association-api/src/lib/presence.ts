import { list, put } from "@vercel/blob";
import { assertBlobConfigured, presenceTtlSeconds } from "./db";
import { parsePeerFromPath, presencePath } from "./peerPath";
import type { PresenceRequest } from "./validate";
import { PRESENCE_SCHEMA_VERSION, type PresencePeer, type PresenceResponse } from "./validate";

export async function upsertAndListPeers(
  request: PresenceRequest,
  now: Date = new Date(),
): Promise<PresenceResponse> {
  assertBlobConfigured();
  const ttlMs = presenceTtlSeconds() * 1000;
  const cutoff = now.getTime() - ttlMs;

  await put(
    presencePath(request.serverHash, request.uuid, request.username),
    JSON.stringify({
      uuid: request.uuid,
      username: request.username,
      clientVersion: request.clientVersion,
      updatedAt: now.toISOString(),
    }),
    {
      access: "public",
      addRandomSuffix: false,
      allowOverwrite: true,
      contentType: "application/json",
      cacheControlMaxAge: 0,
    },
  );

  const byUuid = new Map<string, PresencePeer>();
  let cursor: string | undefined;
  do {
    const listed = await list({
      prefix: `presence/${request.serverHash}/`,
      limit: 250,
      cursor,
    });
    for (const blob of listed.blobs) {
      const uploaded = Date.parse(blob.uploadedAt.toISOString());
      if (!Number.isFinite(uploaded) || uploaded < cutoff) {
        continue;
      }
      const peer = parsePeerFromPath(blob.pathname);
      if (!peer) {
        continue;
      }
      byUuid.set(peer.uuid, peer);
    }
    cursor = listed.hasMore ? listed.cursor : undefined;
  } while (cursor);

  if (!byUuid.has(request.uuid)) {
    byUuid.set(request.uuid, {
      uuid: request.uuid,
      username: request.username.toLowerCase(),
    });
  }

  const peerEntries = Array.from(byUuid.values()).sort((left, right) =>
    left.uuid.localeCompare(right.uuid),
  );
  const peers = peerEntries.map((peer) => peer.uuid);

  return {
    schemaVersion: PRESENCE_SCHEMA_VERSION,
    peers,
    peerEntries,
  };
}

/** Pure helper for unit tests: filter stale peers without I/O. */
export function filterFreshPeerUuids(
  rows: Array<{ player_uuid: string; last_seen: string }>,
  nowMs: number,
  ttlSeconds: number,
): string[] {
  const cutoff = nowMs - ttlSeconds * 1000;
  return rows
    .filter((row) => Date.parse(row.last_seen) >= cutoff)
    .map((row) => row.player_uuid.toLowerCase())
    .filter((uuid, index, all) => all.indexOf(uuid) === index)
    .sort();
}

export function parsePeerFromPathForTest(pathname: string): PresencePeer | null {
  return parsePeerFromPath(pathname);
}
