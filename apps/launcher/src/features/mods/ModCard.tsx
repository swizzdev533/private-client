import { useState } from "react";
import { motion } from "framer-motion";
import {
  ArrowDownToLine,
  Check,
  ChevronDown,
  Clock3,
  Download,
  RefreshCcw,
  ShieldCheck,
} from "lucide-react";
import { Badge, type BadgeTone } from "../../components/common/Badge";
import { Button } from "../../components/common/Button";
import { formatBytes, formatDate, formatDownloads } from "../../lib/format";
import type { ModSummary } from "../../types/contracts";
import { ModIcon } from "./ModIcon";

interface ModCardProps {
  mod: ModSummary;
  busy: boolean;
  onInstall: (mod: ModSummary) => void;
  onUpdateInstalled: (mod: ModSummary) => void;
}

const compatibilityTone: Readonly<Record<ModSummary["compatibility"], BadgeTone>> = {
  COMPATIBLE: "success",
  EXPERIMENTAL: "warning",
  LICENSE_REVIEW: "warning",
  INCOMPATIBLE: "danger",
  DOWNLOAD_UNAVAILABLE: "danger",
};

export function ModCard({ mod, busy, onInstall, onUpdateInstalled }: ModCardProps) {
  const [expanded, setExpanded] = useState(false);
  const canInstall = ["COMPATIBLE", "EXPERIMENTAL"].includes(mod.compatibility);
  const action = mod.updateAvailable
    ? {
        label: "UPDATE",
        icon: <RefreshCcw size={15} />,
        run: () => {
          onUpdateInstalled(mod);
        },
      }
    : mod.installed
      ? {
          label: "INSTALLED",
          icon: <Check size={15} />,
          run: undefined,
        }
      : {
          label: "INSTALL",
          icon: <ArrowDownToLine size={15} />,
          run: () => {
            onInstall(mod);
          },
        };

  return (
    <motion.article
      className={`mod-card ${expanded ? "is-expanded" : ""}`}
      layout
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2 }}
    >
      <div className="mod-card__primary">
        <ModIcon name={mod.name} iconUrl={mod.iconUrl} />
        <div className="mod-card__identity">
          <div className="mod-card__title">
            <h3>{mod.name}</h3>
            {mod.trust === "VERIFIED" ? (
              <ShieldCheck size={14} aria-label="Zweryfikowany projekt" />
            ) : null}
          </div>
          <span>by {mod.author}</span>
        </div>
        <div className="mod-card__badges">
          <Badge tone={mod.trust === "VERIFIED" ? "bright" : "neutral"}>
            {mod.trust.replace("_", " ")}
          </Badge>
          <Badge tone={compatibilityTone[mod.compatibility]}>
            {mod.compatibility.replaceAll("_", " ")}
          </Badge>
        </div>
      </div>

      <p className="mod-card__description">{mod.description}</p>

      <div className="mod-card__meta">
        <span>
          <Download size={12} />
          {formatDownloads(mod.downloads)}
        </span>
        <span>
          <Clock3 size={12} />
          {formatDate(mod.updatedAt)}
        </span>
        <span>{mod.version}</span>
        <span>{formatBytes(mod.fileSize)}</span>
      </div>

      {mod.compatibilityReason ? (
        <p className="mod-card__reason">{mod.compatibilityReason}</p>
      ) : null}

      <div className="mod-card__footer">
        <button
          type="button"
          className="mod-card__details-button"
          onClick={() => {
            setExpanded((value) => !value);
          }}
          aria-expanded={expanded}
        >
          SZCZEGÓŁY
          <ChevronDown
            size={14}
            className={expanded ? "is-rotated" : ""}
            aria-hidden="true"
          />
        </button>
        <Button
          variant={mod.installed && !mod.updateAvailable ? "ghost" : "secondary"}
          size="sm"
          busy={busy}
          disabled={!canInstall || action.run === undefined}
          icon={action.icon}
          onClick={action.run}
        >
          {action.label}
        </Button>
      </div>

      {expanded ? (
        <motion.dl
          className="mod-card__details"
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
        >
          <div>
            <dt>Loader</dt>
            <dd>Forge</dd>
          </div>
          <div>
            <dt>Minecraft</dt>
            <dd>1.8.9</dd>
          </div>
          <div>
            <dt>Środowisko</dt>
            <dd>{mod.environment.replaceAll("_", " + ")}</dd>
          </div>
          <div>
            <dt>Licencja</dt>
            <dd>{mod.license}</dd>
          </div>
          <div>
            <dt>Zależności</dt>
            <dd>{mod.dependencyCount}</dd>
          </div>
          <div>
            <dt>Wydanie</dt>
            <dd>{mod.releaseType}</dd>
          </div>
        </motion.dl>
      ) : null}
    </motion.article>
  );
}
