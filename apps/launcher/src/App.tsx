import { useEffect } from "react";
import { AnimatePresence, useReducedMotion } from "framer-motion";
import { BackgroundScene } from "./components/layout/BackgroundScene";
import { AppShell } from "./components/layout/AppShell";
import { NoticeStack } from "./components/common/NoticeStack";
import { PlayView } from "./features/play/PlayView";
import { ModsView } from "./features/mods/ModsView";
import { SettingsModal } from "./features/settings/SettingsModal";
import { InstallPlanModal } from "./features/mods/InstallPlanModal";
import { OptifineModal } from "./features/mods/OptifineModal";
import { subscribeLauncherEvents } from "./services/launcherApi";
import { subscribeOperationProgress } from "./services/modsApi";
import { useLauncherStore } from "./stores/useLauncherStore";
import { useModsStore } from "./stores/useModsStore";
import { useUiStore } from "./stores/useUiStore";

/**
 * Losing an event subscription means launch progress, profile and mod updates
 * stop arriving for the rest of the session, so it must surface rather than
 * become an unhandled rejection.
 */
function reportSubscriptionFailure(error: unknown) {
  console.error("Launcher event subscription failed", error);
  useUiStore.getState().notify({
    tone: "error",
    title: "Utracono połączenie ze zdarzeniami launchera",
    message: "Uruchom launcher ponownie, aby przywrócić aktualizacje stanu.",
  });
}

export default function App() {
  const systemReducedMotion = useReducedMotion();
  const activeTab = useUiStore((state) => state.activeTab);
  const modal = useUiStore((state) => state.modal);
  const snapshot = useLauncherStore((state) => state.snapshot);
  const initialize = useLauncherStore((state) => state.initialize);
  const applyLaunchState = useLauncherStore((state) => state.applyLaunchState);
  const applyProfile = useLauncherStore((state) => state.applyProfile);
  const refreshLocalState = useModsStore((state) => state.refreshLocalState);
  const searchMods = useModsStore((state) => state.search);
  const applyOperationProgress = useModsStore((state) => state.applyOperationProgress);
  const reducedMotion = Boolean(systemReducedMotion || snapshot?.settings.reducedMotion);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    let disposed = false;
    let stopLauncherEvents: (() => void) | undefined;
    let stopOperationEvents: (() => void) | undefined;

    void subscribeLauncherEvents({
      onLaunchState: applyLaunchState,
      onProfile: applyProfile,
      onModsChanged: () => {
        void Promise.all([refreshLocalState(), searchMods()]);
      },
    })
      .then((dispose) => {
        if (disposed) {
          dispose();
        } else {
          stopLauncherEvents = dispose;
        }
      })
      .catch(reportSubscriptionFailure);

    void subscribeOperationProgress(applyOperationProgress)
      .then((dispose) => {
        if (disposed) {
          dispose();
        } else {
          stopOperationEvents = dispose;
        }
      })
      .catch(reportSubscriptionFailure);

    return () => {
      disposed = true;
      stopLauncherEvents?.();
      stopOperationEvents?.();
    };
  }, [
    applyLaunchState,
    applyOperationProgress,
    applyProfile,
    refreshLocalState,
    searchMods,
  ]);

  return (
    <>
      <BackgroundScene reducedMotion={reducedMotion} />
      <AppShell appVersion={snapshot?.appVersion ?? "1.0.0"}>
        <AnimatePresence mode="wait" initial={false}>
          {activeTab === "play" ? (
            <PlayView key="play" reducedMotion={reducedMotion} />
          ) : (
            <ModsView key="mods" reducedMotion={reducedMotion} />
          )}
        </AnimatePresence>
      </AppShell>
      <NoticeStack />

      <AnimatePresence>
        {modal === "settings" ? <SettingsModal key="settings" /> : null}
        {modal === "install-plan" ? <InstallPlanModal key="install-plan" /> : null}
        {modal === "optifine" ? <OptifineModal key="optifine" /> : null}
      </AnimatePresence>
    </>
  );
}
