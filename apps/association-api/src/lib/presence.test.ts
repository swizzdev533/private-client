import { describe, expect, it } from "vitest";
import { filterFreshPeerUuids } from "./presence";
import { checkRateLimit, resetRateLimits } from "./rateLimit";

describe("filterFreshPeerUuids", () => {
  it("drops stale rows outside the TTL window", () => {
    const now = Date.parse("2026-08-03T12:00:00.000Z");
    const peers = filterFreshPeerUuids(
      [
        {
          player_uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
          last_seen: "2026-08-03T11:59:30.000Z",
        },
        {
          player_uuid: "11111111-2222-3333-4444-555555555555",
          last_seen: "2026-08-03T11:58:00.000Z",
        },
      ],
      now,
      45,
    );
    expect(peers).toEqual(["aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"]);
  });
});

describe("checkRateLimit", () => {
  it("allows traffic under the limit and blocks after", () => {
    resetRateLimits();
    const now = 1_000_000;
    expect(checkRateLimit("uuid:test", 2, now)).toBe(true);
    expect(checkRateLimit("uuid:test", 2, now + 1)).toBe(true);
    expect(checkRateLimit("uuid:test", 2, now + 2)).toBe(false);
  });
});
