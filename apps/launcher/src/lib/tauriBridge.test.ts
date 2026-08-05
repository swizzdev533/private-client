import { beforeEach, describe, expect, it, vi } from "vitest";
import { z } from "zod";
import { invokeValidated, listenValidated, localAssetUrl } from "./tauriBridge";
import { DomainError, updateStatusSchema } from "../types/contracts";
import * as demoBackend from "./demoBackend";

/**
 * The IPC bridge is a trust boundary: every backend response and event payload
 * is untrusted until a schema accepts it. These tests pin that behaviour, and
 * that a rejection surfaces as a typed DomainError rather than a raw throw.
 */
describe("IPC validation boundary", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    demoBackend.resetDemoBackend();
  });

  it("rejects a backend response that does not match the schema", async () => {
    vi.spyOn(demoBackend, "invokeDemo").mockResolvedValue({
      available: "yes",
      currentVersion: "1.0.0",
    });

    await expect(invokeValidated("check_for_update", updateStatusSchema)).rejects.toBeInstanceOf(
      DomainError,
    );
  });

  it("rejects release notes that exceed the documented bound", async () => {
    vi.spyOn(demoBackend, "invokeDemo").mockResolvedValue({
      available: true,
      currentVersion: "1.0.0",
      availableVersion: "1.1.0",
      notes: "x".repeat(2001),
      publishedAt: null,
    });

    await expect(invokeValidated("check_for_update", updateStatusSchema)).rejects.toBeInstanceOf(
      DomainError,
    );
  });

  it("passes a well-formed response through unchanged", async () => {
    const payload = {
      available: true,
      currentVersion: "1.0.0",
      availableVersion: "1.1.0",
      notes: "Poprawki",
      publishedAt: "2026-08-01T00:00:00Z",
    };
    vi.spyOn(demoBackend, "invokeDemo").mockResolvedValue(payload);

    await expect(invokeValidated("check_for_update", updateStatusSchema)).resolves.toEqual(payload);
  });

  it("preserves a typed backend error instead of masking it", async () => {
    vi.spyOn(demoBackend, "invokeDemo").mockRejectedValue({
      id: "OperationBlockedWhileRunning",
      title: "Gra jest aktywna",
      message: "Nie można aktualizować w trakcie gry.",
      resolution: "Zamknij grę.",
      logPath: null,
    });

    await expect(invokeValidated("install_update", z.unknown())).rejects.toMatchObject({
      id: "OperationBlockedWhileRunning",
      resolution: "Zamknij grę.",
    });
  });

  it("labels an untyped throw rather than leaking it to the UI", async () => {
    vi.spyOn(demoBackend, "invokeDemo").mockRejectedValue(new Error("socket hang up"));

    await expect(invokeValidated("check_for_update", updateStatusSchema)).rejects.toMatchObject({
      id: "UnexpectedError",
    });
  });

  it("drops a malformed event payload instead of invoking the handler", async () => {
    // Capture the wrapper listenValidated registers, so the assertion is about
    // the validation wrapper rather than the demo backend's own dispatch.
    const captured: { deliver: ((payload: unknown) => void) | null } = { deliver: null };
    vi.spyOn(demoBackend, "subscribeDemo").mockImplementation((_name, listener) => {
      captured.deliver = listener;
      return () => undefined;
    });

    const handler = vi.fn();
    await listenValidated("launcher://mods-changed", z.object({ reason: z.string() }), handler);

    captured.deliver?.({ reason: 42 });
    expect(handler).not.toHaveBeenCalled();

    captured.deliver?.({ reason: "installed" });
    expect(handler).toHaveBeenCalledWith({ reason: "installed" });
  });

  it("refuses to turn a local path into a url outside the Tauri runtime", () => {
    expect(localAssetUrl("C:/Users/player/skin.png")).toBeNull();
    expect(localAssetUrl(null)).toBeNull();
  });

  it("passes remote and inline sources through untouched", () => {
    expect(localAssetUrl("https://textures.minecraft.net/a.png")).toBe(
      "https://textures.minecraft.net/a.png",
    );
    expect(localAssetUrl("data:image/png;base64,AAA")).toBe("data:image/png;base64,AAA");
  });
});
