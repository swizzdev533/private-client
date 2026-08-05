import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauriRuntime } from "../lib/tauriBridge";

async function withCurrentWindow(
  action: (window: ReturnType<typeof getCurrentWindow>) => Promise<void>,
): Promise<void> {
  if (!isTauriRuntime) {
    return;
  }
  await action(getCurrentWindow());
}

export const windowApi = {
  minimize: () =>
    withCurrentWindow(async (window) => {
      await window.minimize();
    }),
  toggleMaximize: () =>
    withCurrentWindow(async (window) => {
      await window.toggleMaximize();
    }),
  close: () =>
    withCurrentWindow(async (window) => {
      await window.close();
    }),
};
