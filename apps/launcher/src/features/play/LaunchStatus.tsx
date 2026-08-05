import { Box, Check, CircleDashed, Cpu, Gamepad2, Layers3 } from "lucide-react";
import { motion } from "framer-motion";
import { Badge } from "../../components/common/Badge";
import { ProgressBar } from "../../components/common/ProgressBar";
import {
  launchStateLabels,
  type InstanceSummary,
  type LaunchProgress,
  type LaunchState,
} from "../../types/contracts";

interface LaunchStatusProps {
  launch: LaunchProgress;
  instance: InstanceSummary;
}

const orderedStates: LaunchState[] = [
  "IDLE",
  "VALIDATING",
  "CHECKING_RUNTIME",
  "PREPARING_INSTANCE",
  "VERIFYING_GAME_FILES",
  "INSTALLING_GAME_FILES",
  "VERIFYING_FORGE",
  "INSTALLING_FORGE",
  "CHECKING_REQUIRED_MODS",
  "APPLYING_PENDING_CHANGES",
  "BUILDING_LAUNCH_COMMAND",
  "LAUNCHING",
  "RUNNING",
  "STOPPING",
  "EXITED",
  "FAILED",
];

const stages = [
  {
    label: "Runtime",
    icon: Cpu,
    states: ["VALIDATING", "CHECKING_RUNTIME"],
  },
  {
    label: "Instancja",
    icon: Box,
    states: ["PREPARING_INSTANCE", "VERIFYING_GAME_FILES", "INSTALLING_GAME_FILES"],
  },
  {
    label: "Forge + mody",
    icon: Layers3,
    states: [
      "VERIFYING_FORGE",
      "INSTALLING_FORGE",
      "CHECKING_REQUIRED_MODS",
      "APPLYING_PENDING_CHANGES",
    ],
  },
  {
    label: "Start",
    icon: Gamepad2,
    states: ["BUILDING_LAUNCH_COMMAND", "LAUNCHING", "RUNNING"],
  },
] as const;

function stateIndex(state: LaunchState): number {
  return orderedStates.indexOf(state);
}

export function LaunchStatus({ launch, instance }: LaunchStatusProps) {
  const currentIndex = stateIndex(launch.state);
  const isActive = !["IDLE", "EXITED", "FAILED"].includes(launch.state);

  return (
    <section
      className="launch-status"
      aria-label="Stan uruchamiania"
      data-testid="launch-state"
      data-state={launch.state}
    >
      <header className="launch-status__header">
        <div>
          <span className="eyebrow">SYSTEM</span>
          <h2>Stan instancji</h2>
        </div>
        <Badge
          tone={
            launch.state === "FAILED"
              ? "danger"
              : launch.state === "RUNNING"
                ? "success"
                : instance.healthy
                  ? "bright"
                  : "warning"
          }
          dot
        >
          {launch.state === "FAILED"
            ? "WYMAGA UWAGI"
            : launch.state === "RUNNING"
              ? "RUNNING"
              : instance.healthy
                ? "GOTOWA"
                : "NAPRAWA"}
        </Badge>
      </header>

      <div className="launch-status__current">
        <div className={`status-orb status-orb--${launch.state.toLowerCase()}`}>
          {launch.state === "RUNNING" ? <Check size={20} /> : <CircleDashed size={20} />}
        </div>
        <div>
          <strong>{launchStateLabels[launch.state]}</strong>
          <span>{launch.message}</span>
        </div>
      </div>

      {isActive && launch.state !== "RUNNING" ? (
        <ProgressBar value={launch.progress} label="Postęp przygotowania" />
      ) : null}

      {launch.state === "FAILED" ? (
        <div className="launch-status__failure" data-testid="launch-failure">
          <p>{launch.message}</p>
          <dl>
            {launch.errorId ? (
              <div>
                <dt>Kod błędu</dt>
                <dd>{launch.errorId}</dd>
              </div>
            ) : null}
            {launch.logPath ? (
              <div>
                <dt>Log</dt>
                <dd className="launch-status__log-path">{launch.logPath}</dd>
              </div>
            ) : null}
          </dl>
        </div>
      ) : null}

      <div className="launch-stages">
        {stages.map((stage) => {
          const Icon = stage.icon;
          const firstState = stage.states[0] as LaunchState;
          const lastState = stage.states.at(-1) as LaunchState;
          const active = stage.states.some((state) => state === launch.state);
          const complete =
            currentIndex > stateIndex(lastState) ||
            launch.state === "RUNNING" ||
            (launch.state === "EXITED" && stateIndex(firstState) > 0);

          return (
            <div
              className={`launch-stage ${active ? "is-active" : ""} ${complete ? "is-complete" : ""}`}
              key={stage.label}
            >
              <span className="launch-stage__icon">
                {complete ? <Check size={13} /> : <Icon size={13} />}
              </span>
              <span>{stage.label}</span>
              {active ? (
                <motion.span
                  className="launch-stage__pulse"
                  layoutId="launch-stage-pulse"
                />
              ) : null}
            </div>
          );
        })}
      </div>

      <dl className="instance-facts">
        <div>
          <dt>Minecraft</dt>
          <dd>{instance.minecraftVersion}</dd>
        </div>
        <div>
          <dt>Forge</dt>
          <dd>{instance.forgeVersion}</dd>
        </div>
        <div>
          <dt>Runtime</dt>
          <dd>{instance.javaLabel ?? "Do wykrycia"}</dd>
        </div>
        <div>
          <dt>Oczekujące zmiany</dt>
          <dd>{instance.pendingOperations}</dd>
        </div>
      </dl>
    </section>
  );
}
