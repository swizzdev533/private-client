import { create } from "zustand";
import { launcherApi } from "../services/launcherApi";
import type {
  LauncherSettings,
  LauncherSnapshot,
  LaunchProgress,
  PlayerProfile,
  UpdateStatus,
} from "../types/contracts";
import { DomainError } from "../types/contracts";
import { useUiStore } from "./useUiStore";

interface LauncherState {
  snapshot: LauncherSnapshot | null;
  initializing: boolean;
  actionPending: boolean;
  update: UpdateStatus | null;
  updateChecking: boolean;
  updateInstalling: boolean;
  checkForUpdate: (options?: { silent?: boolean }) => Promise<void>;
  installUpdate: () => Promise<void>;
  dismissUpdate: () => void;
  initialize: () => Promise<void>;
  applyLaunchState: (launch: LaunchProgress) => void;
  applyProfile: (profile: PlayerProfile | null) => void;
  launchOrFocus: () => Promise<void>;
  cancelLaunch: () => Promise<void>;
  stopGame: () => Promise<void>;
  saveSettings: (settings: LauncherSettings) => Promise<boolean>;
  openLogs: () => Promise<void>;
  exportLogs: () => Promise<void>;
}

function notifyError(error: unknown): void {
  const domain =
    error instanceof DomainError
      ? error
      : new DomainError({
          id: "UnexpectedError",
          title: "The operation failed",
          message: error instanceof Error ? error.message : String(error),
          resolution: null,
          logPath: null,
        });
  useUiStore.getState().notify({
    tone: "error",
    title: `${domain.title} · ${domain.id}`,
    message: domain.resolution ? `${domain.message} ${domain.resolution}` : domain.message,
  });
}

export const useLauncherStore = create<LauncherState>((set, get) => ({
  snapshot: null,
  initializing: true,
  actionPending: false,
  update: null,
  updateChecking: false,
  updateInstalling: false,
  initialize: async () => {
    set({ initializing: true });
    try {
      const snapshot = await launcherApi.snapshot();
      set({ snapshot, initializing: false });
      // The autoUpdateChecks setting gates background network access only; the
      // manual check in Settings stays available either way.
      if (snapshot.settings.autoUpdateChecks) {
        void get().checkForUpdate({ silent: true });
      }
    } catch (error) {
      set({ initializing: false });
      notifyError(error);
    }
  },
  checkForUpdate: async (options) => {
    const silent = options?.silent ?? false;
    if (get().updateChecking) {
      return;
    }
    set({ updateChecking: true });
    try {
      const update = await launcherApi.checkForUpdate();
      set({ update });
      if (!silent && !update.available) {
        useUiStore.getState().notify({
          tone: "neutral",
          title: "The launcher is up to date",
          message: `Installed version  is the newest.`,
        });
      }
    } catch (error) {
      // A background check must never interrupt the user; an unreachable
      // release host simply means "updates unavailable" for this session.
      if (!silent) {
        notifyError(error);
      }
    } finally {
      set({ updateChecking: false });
    }
  },
  installUpdate: async () => {
    if (get().updateInstalling) {
      return;
    }
    set({ updateInstalling: true });
    try {
      const result = await launcherApi.installUpdate();
      useUiStore.getState().notify({
        tone: "success",
        title: "Update ready",
        message: result.message,
      });
    } catch (error) {
      notifyError(error);
    } finally {
      set({ updateInstalling: false });
    }
  },
  dismissUpdate: () => {
    set({ update: null });
  },
  applyLaunchState: (launch) => {
    set((state) => ({
      snapshot: state.snapshot ? { ...state.snapshot, launch } : null,
    }));
  },
  applyProfile: (profile) => {
    set((state) => ({
      snapshot: state.snapshot ? { ...state.snapshot, profile } : null,
    }));
  },
  launchOrFocus: async () => {
    const state = get().snapshot?.launch.state;
    set({ actionPending: true });
    try {
      const result =
        state === "RUNNING" ? await launcherApi.focusGame() : await launcherApi.launch();
      useUiStore.getState().notify({
        tone: "success",
        title: state === "RUNNING" ? "The game is running" : "Starting",
        message: result.message,
      });
    } catch (error) {
      notifyError(error);
    } finally {
      set({ actionPending: false });
    }
  },
  cancelLaunch: async () => {
    set({ actionPending: true });
    try {
      const result = await launcherApi.cancelLaunch();
      useUiStore.getState().notify({
        tone: "neutral",
        title: "Operation cancelled",
        message: result.message,
      });
    } catch (error) {
      notifyError(error);
    } finally {
      set({ actionPending: false });
    }
  },
  stopGame: async () => {
    set({ actionPending: true });
    try {
      const result = await launcherApi.stop();
      useUiStore.getState().notify({
        tone: "neutral",
        title: "Stopping the game",
        message: result.message,
      });
    } catch (error) {
      notifyError(error);
    } finally {
      set({ actionPending: false });
    }
  },
  saveSettings: async (settings) => {
    try {
      const validated = await launcherApi.saveSettings(settings);
      set((state) => ({
        snapshot: state.snapshot ? { ...state.snapshot, settings: validated } : null,
      }));
      useUiStore.getState().notify({
        tone: "success",
        title: "Settings saved",
        message: "The changes will apply the next time the game starts.",
      });
      return true;
    } catch (error) {
      notifyError(error);
      return false;
    }
  },
  openLogs: async () => {
    try {
      const result = await launcherApi.openLogs();
      useUiStore.getState().notify({
        tone: "neutral",
        title: "Local logs",
        message: result.message,
      });
    } catch (error) {
      notifyError(error);
    }
  },
  exportLogs: async () => {
    try {
      const result = await launcherApi.exportLogs();
      useUiStore.getState().notify({
        tone: "success",
        title: "Export ready",
        message: result.message,
      });
    } catch (error) {
      notifyError(error);
    }
  },
}));
