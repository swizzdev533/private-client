import { z } from "zod";
import {
  commandResultSchema,
  launcherSettingsSchema,
  launcherSnapshotSchema,
  launchProgressSchema,
  profileUpdatedEventSchema,
  updateStatusSchema,
  type LauncherSettings,
} from "../types/contracts";
import { invokeValidated, listenValidated } from "../lib/tauriBridge";

const modsChangedSchema = z.object({
  reason: z.string(),
  projectId: z.string().optional(),
});

export const launcherApi = {
  snapshot: () => invokeValidated("get_launcher_snapshot", launcherSnapshotSchema),
  launch: () => invokeValidated("launch_game", commandResultSchema),
  cancelLaunch: () => invokeValidated("cancel_launch", commandResultSchema),
  stop: () => invokeValidated("stop_game", commandResultSchema),
  focusGame: () => invokeValidated("focus_game_window", commandResultSchema),
  saveSettings: (settings: LauncherSettings) =>
    invokeValidated("save_launcher_settings", launcherSettingsSchema, {
      settings,
    }),
  openLogs: () => invokeValidated("open_logs_directory", commandResultSchema),
  exportLogs: () => invokeValidated("export_logs", commandResultSchema),
  checkForUpdate: () => invokeValidated("check_for_update", updateStatusSchema),
  installUpdate: () => invokeValidated("install_update", commandResultSchema),
};

export interface LauncherEventHandlers {
  onLaunchState: (payload: z.infer<typeof launchProgressSchema>) => void;
  onProfile: (payload: z.infer<typeof profileUpdatedEventSchema>) => void;
  onModsChanged: () => void;
}

export async function subscribeLauncherEvents(
  handlers: LauncherEventHandlers,
): Promise<() => void> {
  // allSettled rather than all: with Promise.all a later rejection discards the
  // UnlistenFn of the subscriptions that already succeeded, leaking them for the
  // lifetime of the process.
  const settled = await Promise.allSettled([
    listenValidated(
      "launcher://launch-state",
      launchProgressSchema,
      handlers.onLaunchState,
    ),
    listenValidated(
      "launcher://profile-updated",
      profileUpdatedEventSchema,
      handlers.onProfile,
    ),
    listenValidated("launcher://mods-changed", modsChangedSchema, () => {
      handlers.onModsChanged();
    }),
  ]);

  const unlisten = settled
    .filter((result) => result.status === "fulfilled")
    .map((result) => result.value);
  const failure = settled.find((result) => result.status === "rejected");
  if (failure) {
    unlisten.forEach((dispose) => {
      dispose();
    });
    throw failure.reason instanceof Error
      ? failure.reason
      : new Error("Could not subscribe to launcher events");
  }

  return () => {
    unlisten.forEach((dispose) => {
      dispose();
    });
  };
}
