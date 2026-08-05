import { lazy, Suspense } from "react";
import { motion } from "framer-motion";
import { ArrowUpRight, Play, RotateCcw, Square } from "lucide-react";
import { Button } from "../../components/common/Button";
import { LaunchStatus } from "./LaunchStatus";
import { useLauncherStore } from "../../stores/useLauncherStore";

const PlayerSkinViewer = lazy(async () => {
  const module = await import("./PlayerSkinViewer");
  return { default: module.PlayerSkinViewer };
});

interface PlayViewProps {
  reducedMotion: boolean;
}

function PlaySkeleton() {
  return (
    <div className="play-layout play-layout--loading" aria-label="Loading PLAY">
      <div>
        <div className="skeleton" style={{ width: 120, height: 16 }} />
        <div className="skeleton" style={{ width: "78%", height: 72, marginTop: 20 }} />
        <div className="skeleton" style={{ width: 280, height: 44, marginTop: 28 }} />
      </div>
      <div className="skeleton play-skeleton__model" />
    </div>
  );
}

export function PlayView({ reducedMotion }: PlayViewProps) {
  const snapshot = useLauncherStore((state) => state.snapshot);
  const actionPending = useLauncherStore((state) => state.actionPending);
  const launchOrFocus = useLauncherStore((state) => state.launchOrFocus);
  const cancelLaunch = useLauncherStore((state) => state.cancelLaunch);
  const stopGame = useLauncherStore((state) => state.stopGame);

  if (!snapshot) {
    return (
      <div className="page">
        <div className="page__inner">
          <PlaySkeleton />
        </div>
      </div>
    );
  }

  const { profile, launch, instance } = snapshot;
  const running = launch.state === "RUNNING";
  const working = !["IDLE", "EXITED", "FAILED", "RUNNING"].includes(launch.state);
  // The prepare/launch flow stays quiet — the PLAY button already reports it.
  // Only a failure needs the panel, for the error code and crash-log pointer.
  const showStatus = launch.state === "FAILED";

  return (
    <motion.div
      className="page"
      initial={reducedMotion ? false : { opacity: 0, x: -16 }}
      animate={{ opacity: 1, x: 0 }}
      exit={reducedMotion ? {} : { opacity: 0, x: 12 }}
      transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
    >
      <div className="page__inner play-page">
        <div className="play-layout">
          <section className="play-hero">
            <div className="play-hero__title">
              <h1>
                PRIVATE
                <br />
                <em>CLIENT</em>
              </h1>
              <p>Private Client is a new way to rediscover Minecraft.</p>
            </div>

            <div className="play-actions">
              <Button
                variant="primary"
                size="lg"
                className="play-button"
                data-testid="launch-action"
                busy={actionPending || working}
                disabled={working && !launch.canCancel}
                onClick={() => {
                  void launchOrFocus();
                }}
                icon={
                  running ? (
                    <ArrowUpRight size={18} />
                  ) : (
                    <Play size={18} fill="currentColor" />
                  )
                }
              >
                {running
                  ? "OPEN GAME"
                  : working
                    ? launch.state.replaceAll("_", " ")
                    : "PLAY"}
              </Button>

              {launch.canCancel ? (
                <Button
                  variant="ghost"
                  size="lg"
                  onClick={() => {
                    void cancelLaunch();
                  }}
                  icon={<RotateCcw size={17} />}
                >
                  ANULUJ
                </Button>
              ) : null}

              {running ? (
                <Button
                  variant="ghost"
                  size="lg"
                  onClick={() => {
                    void stopGame();
                  }}
                  icon={<Square size={15} />}
                >
                  ZAMKNIJ
                </Button>
              ) : null}
            </div>

            {showStatus ? (
              <LaunchStatus launch={launch} instance={instance} />
            ) : (
              // Keeps the launch state observable (tests, tooling) without
              // rendering the panel.
              <span hidden data-testid="launch-state" data-state={launch.state} />
            )}
          </section>

          <Suspense
            fallback={
              <div
                className="skeleton play-skeleton__model"
                aria-label="Loading player model"
              />
            }
          >
            <PlayerSkinViewer profile={profile} reducedMotion={reducedMotion} />
          </Suspense>
        </div>
      </div>
    </motion.div>
  );
}
