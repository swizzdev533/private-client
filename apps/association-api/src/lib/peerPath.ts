import type { PresencePeer } from "./validate";

export function presencePath(
  serverHash: string,
  uuid: string,
  username: string,
): string {
  return `presence/${serverHash}/${uuid}__${username.toLowerCase()}.json`;
}

export function parsePeerFromPath(pathname: string): PresencePeer | null {
  const file = pathname.split("/").pop() ?? "";
  const base = file.replace(/\.json$/i, "").toLowerCase();
  if (!base) {
    return null;
  }
  const separator = base.indexOf("__");
  const uuid = separator >= 0 ? base.slice(0, separator) : base;
  const username = separator >= 0 ? base.slice(separator + 2) : "";
  if (!/^[0-9a-f-]{36}$/.test(uuid)) {
    return null;
  }
  if (username && !/^[a-z0-9_]{1,16}$/.test(username)) {
    return null;
  }
  return { uuid, username: username || undefined };
}
