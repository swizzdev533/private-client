import { describe, expect, it } from "vitest";
import { parsePeerFromPathForTest } from "./presence";
import { presenceRequestSchema, presenceResponseSchema } from "./validate";

describe("presenceRequestSchema", () => {
  it("accepts a valid payload and lowercases uuid", () => {
    const parsed = presenceRequestSchema.parse({
      schemaVersion: 1,
      uuid: "AAAAAAAA-BBBB-4CCC-8DDD-EEEEEEEEEEEE",
      username: "Zinox5",
      serverHash: "a".repeat(64),
      clientVersion: "1.0.0",
    });
    expect(parsed.uuid).toBe("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee");
  });

  it("rejects invalid server hashes", () => {
    const result = presenceRequestSchema.safeParse({
      schemaVersion: 1,
      uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
      username: "Zinox5",
      serverHash: "not-a-hash",
      clientVersion: "1.0.0",
    });
    expect(result.success).toBe(false);
  });

  it("rejects usernames with illegal characters", () => {
    const result = presenceRequestSchema.safeParse({
      schemaVersion: 1,
      uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
      username: "bad name!",
      serverHash: "b".repeat(64),
      clientVersion: "1.0.0",
    });
    expect(result.success).toBe(false);
  });
});

describe("presenceResponseSchema", () => {
  it("accepts peerEntries with usernames", () => {
    const parsed = presenceResponseSchema.parse({
      schemaVersion: 1,
      peers: ["aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"],
      peerEntries: [
        {
          uuid: "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
          username: "Zinox5",
        },
      ],
    });
    expect(parsed.peerEntries?.[0]?.username).toBe("Zinox5");
  });
});

describe("parsePeerFromPathForTest", () => {
  it("reads uuid and username from path", () => {
    expect(
      parsePeerFromPathForTest(
        "presence/abc/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee__zinox5.json",
      ),
    ).toEqual({
      uuid: "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
      username: "zinox5",
    });
  });
});
