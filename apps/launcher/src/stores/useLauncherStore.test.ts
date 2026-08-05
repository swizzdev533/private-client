import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLauncherStore } from "./useLauncherStore";
import { useUiStore } from "./useUiStore";
import { launcherApi } from "../services/launcherApi";
import { demoSnapshot } from "../lib/demoFixtures";
import { DomainError, type LauncherSnapshot, type UpdateStatus } from "../types/contracts";

function snapshotWith(autoUpdateChecks: boolean): LauncherSnapshot {
  const base = structuredClone(demoSnapshot);
  return { ...base, settings: { ...base.settings, autoUpdateChecks } };
}

const availableUpdate: UpdateStatus = {
  available: true,
  currentVersion: "1.0.0",
  availableVersion: "1.1.0",
  notes: "Stability fixes",
  publishedAt: "2026-08-01T00:00:00Z",
};

const currentUpdate: UpdateStatus = {
  available: false,
  currentVersion: "1.0.0",
  availableVersion: null,
  notes: null,
  publishedAt: null,
};

function resetStores(): void {
  useLauncherStore.setState({
    snapshot: null,
    initializing: true,
    actionPending: false,
    update: null,
    updateChecking: false,
    updateInstalling: false,
  });
  useUiStore.setState({ activeTab: "play", modsView: "library", modal: null, notices: [] });
}

describe("launcher store update flow", () => {
  beforeEach(() => {
    resetStores();
    vi.restoreAllMocks();
  });

  it("checks for updates on startup when the setting is enabled", async () => {
    vi.spyOn(launcherApi, "snapshot").mockResolvedValue(snapshotWith(true));
    const check = vi.spyOn(launcherApi, "checkForUpdate").mockResolvedValue(availableUpdate);

    await useLauncherStore.getState().initialize();
    // The startup check is fired without awaiting so it cannot delay first paint.
    await vi.waitFor(() => {
      expect(check).toHaveBeenCalledTimes(1);
    });
    expect(useLauncherStore.getState().update).toEqual(availableUpdate);
  });

  it("does not touch the network on startup when the setting is disabled", async () => {
    vi.spyOn(launcherApi, "snapshot").mockResolvedValue(snapshotWith(false));
    const check = vi.spyOn(launcherApi, "checkForUpdate").mockResolvedValue(availableUpdate);

    await useLauncherStore.getState().initialize();

    expect(check).not.toHaveBeenCalled();
    expect(useLauncherStore.getState().update).toBeNull();
  });

  it("keeps a failing background check silent", async () => {
    vi.spyOn(launcherApi, "checkForUpdate").mockRejectedValue(
      new DomainError({
        id: "NetworkUnavailable",
        title: "No connection",
        message: "The update host is unreachable.",
        resolution: null,
        logPath: null,
      }),
    );

    await useLauncherStore.getState().checkForUpdate({ silent: true });

    expect(useUiStore.getState().notices).toHaveLength(0);
    expect(useLauncherStore.getState().updateChecking).toBe(false);
  });

  it("surfaces a failing manual check to the user", async () => {
    vi.spyOn(launcherApi, "checkForUpdate").mockRejectedValue(
      new DomainError({
        id: "NetworkUnavailable",
        title: "No connection",
        message: "The update host is unreachable.",
        resolution: null,
        logPath: null,
      }),
    );

    await useLauncherStore.getState().checkForUpdate();

    const notices = useUiStore.getState().notices;
    expect(notices).toHaveLength(1);
    expect(notices[0]?.tone).toBe("error");
  });

  it("confirms an up-to-date launcher only on a manual check", async () => {
    vi.spyOn(launcherApi, "checkForUpdate").mockResolvedValue(currentUpdate);

    await useLauncherStore.getState().checkForUpdate({ silent: true });
    expect(useUiStore.getState().notices).toHaveLength(0);

    await useLauncherStore.getState().checkForUpdate();
    expect(useUiStore.getState().notices).toHaveLength(1);
  });

  it("ignores a second check while one is already in flight", async () => {
    let release = (): void => undefined;
    const check = vi.spyOn(launcherApi, "checkForUpdate").mockImplementation(
      () =>
        new Promise<UpdateStatus>((resolve) => {
          release = () => {
            resolve(currentUpdate);
          };
        }),
    );

    const first = useLauncherStore.getState().checkForUpdate({ silent: true });
    const second = useLauncherStore.getState().checkForUpdate({ silent: true });
    release();
    await Promise.all([first, second]);

    expect(check).toHaveBeenCalledTimes(1);
  });

  it("reports a rejected install without clearing the pending update", async () => {
    useLauncherStore.setState({ update: availableUpdate });
    vi.spyOn(launcherApi, "installUpdate").mockRejectedValue(
      new DomainError({
        id: "OperationBlockedWhileRunning",
        title: "The game is running",
        message: "The launcher will not update while the game is running.",
        resolution: null,
        logPath: null,
      }),
    );

    await useLauncherStore.getState().installUpdate();

    expect(useUiStore.getState().notices[0]?.tone).toBe("error");
    expect(useLauncherStore.getState().update).toEqual(availableUpdate);
    expect(useLauncherStore.getState().updateInstalling).toBe(false);
  });

  it("dismisses a pending update without installing it", () => {
    useLauncherStore.setState({ update: availableUpdate });
    const install = vi.spyOn(launcherApi, "installUpdate");

    useLauncherStore.getState().dismissUpdate();

    expect(useLauncherStore.getState().update).toBeNull();
    expect(install).not.toHaveBeenCalled();
  });
});
