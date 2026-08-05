import { describe, expect, it } from "vitest";
import {
  launcherSettingsSchema,
  launchProgressSchema,
  modSearchRequestSchema,
  modSearchResponseSchema,
  modSummarySchema,
  profileUpdatedEventSchema,
  launcherSnapshotSchema,
} from "./contracts";
import { demoSnapshot } from "../lib/demoFixtures";

const baseMod = {
  id: "mod-1",
  projectId: "project-1",
  versionId: "v-1",
  name: "Test Mod",
  author: "Author",
  description: "Desc",
  iconUrl: null as string | null,
  version: "1.0.0",
  releaseType: "release",
  downloads: 100,
  updatedAt: "2024-05-18T14:32:00+00:00",
  minecraftVersion: "1.8.9",
  loader: "forge",
  environment: "client",
  license: "MIT",
  fileSize: 1024,
  dependencyCount: 0,
  trust: "FROM_MODRINTH",
  compatibility: "COMPATIBLE",
  compatibilityReason: null,
  installed: false,
  installedVersion: null,
  updateAvailable: false,
  required: false,
};

describe("frontend contracts", () => {
  it("rejects a snapshot whose channel is not a known build channel", () => {
    // A typo'd or attacker-supplied channel must not render as the stable UI:
    // the badge is the only in-app way to tell the two installs apart.
    const base = { ...demoSnapshot, channel: "stabel" };
    expect(launcherSnapshotSchema.safeParse(base).success).toBe(false);
    expect(launcherSnapshotSchema.safeParse({ ...demoSnapshot, channel: "" }).success).toBe(false);
    expect(launcherSnapshotSchema.safeParse({ ...demoSnapshot, channel: "beta" }).success).toBe(
      true,
    );
  });

  it("rejects an invalid launch progress", () => {
    const result = launchProgressSchema.safeParse({
      state: "RUNNING",
      message: "The game is running",
      progress: 140,
      canCancel: false,
      errorId: null,
      logPath: null,
    });
    expect(result.success).toBe(false);
  });

  it("rejects memory ranges with max lower than min", () => {
    const result = launcherSettingsSchema.safeParse({
      schemaVersion: 1,
      javaPath: null,
      memoryMinMb: 4096,
      memoryMaxMb: 2048,
      reducedMotion: false,
      autoUpdateChecks: true,
      downloadConcurrency: 3,
    });
    expect(result.success).toBe(false);
  });

  it("pins Modrinth search to a typed, bounded request", () => {
    expect(
      modSearchRequestSchema.parse({
        query: "Patcher",
        sort: "relevance",
        trust: "verified",
        page: 0,
      }),
    ).toEqual({
      query: "Patcher",
      sort: "relevance",
      trust: "verified",
      page: 0,
    });
  });

  it("accepts a null profile-updated payload after logout", () => {
    expect(profileUpdatedEventSchema.parse(null)).toBeNull();
  });

  it("parses mod summaries with RFC3339 timezone offset dates (+00:00)", () => {
    const validMod = {
      id: "mod-1",
      projectId: "project-1",
      versionId: "v-1",
      name: "Test Mod",
      author: "Author",
      description: "Desc",
      iconUrl: null,
      version: "1.0.0",
      releaseType: "release",
      downloads: 100,
      updatedAt: "2024-05-18T14:32:00+00:00",
      minecraftVersion: "1.8.9",
      loader: "forge",
      environment: "client",
      license: "MIT",
      fileSize: 1024,
      dependencyCount: 0,
      trust: "FROM_MODRINTH",
      compatibility: "COMPATIBLE",
      compatibilityReason: null,
      installed: false,
      installedVersion: null,
      updateAvailable: false,
      required: false,
    };
    expect(modSummarySchema.safeParse(validMod).success).toBe(true);
  });

  it("accepts an iconless project instead of rejecting the record", () => {
    // Modrinth returns "" for projects without an icon.
    const parsed = modSummarySchema.parse({ ...baseMod, iconUrl: "" });
    expect(parsed.iconUrl).toBeNull();
  });

  it("strips non-https icon urls before they reach an img src", () => {
    for (const hostile of [
      "javascript:alert(1)",
      "file:///c:/windows/win.ini",
      "http://insecure.example/icon.png",
    ]) {
      expect(modSummarySchema.parse({ ...baseMod, iconUrl: hostile }).iconUrl).toBeNull();
    }
    const safe = "https://cdn.modrinth.com/icon.png";
    expect(modSummarySchema.parse({ ...baseMod, iconUrl: safe }).iconUrl).toBe(safe);
  });

  it("keeps the rest of a result page when one row is malformed", () => {
    const parsed = modSearchResponseSchema.parse({
      query: "patcher",
      results: [
        baseMod,
        { ...baseMod, id: "broken", downloads: -5 },
        { ...baseMod, id: "mod-2", projectId: "project-2" },
      ],
      page: 0,
      hasMore: false,
      fromCache: false,
      offline: false,
    });
    expect(parsed.results.map((mod) => mod.id)).toEqual(["mod-1", "mod-2"]);
  });
});
