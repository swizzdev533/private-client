import { useState } from "react";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  Calendar,
  ChevronDown,
  FileArchive,
  Hash,
  RefreshCcw,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { Badge } from "../../components/common/Badge";
import { Button } from "../../components/common/Button";
import { formatBytes, formatDate } from "../../lib/format";
import type { InstalledMod } from "../../types/contracts";
import { ModIcon } from "./ModIcon";

interface InstalledModCardProps {
  mod: InstalledMod;
  busy: boolean;
  onRemove: (mod: InstalledMod) => void;
  onUpdate: (mod: InstalledMod) => void;
}

export function InstalledModCard({ mod, busy, onRemove, onUpdate }: InstalledModCardProps) {
  const [expanded, setExpanded] = useState(false);
  const removalBlocked = mod.required || mod.dependents.length > 0;

  return (
    <motion.article
      className={`mod-card installed-mod-card ${expanded ? "is-expanded" : ""}`}
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
            {mod.required ? <ShieldCheck size={14} aria-label="Required component" /> : null}
          </div>
          <span>by {mod.author}</span>
        </div>
        <div className="mod-card__badges">
          <Badge tone={mod.required ? "bright" : "neutral"}>
            {mod.required ? "REQUIRED" : mod.provider.replace("-", " ")}
          </Badge>
          {mod.updateAvailable ? <Badge tone="warning">UPDATE AVAILABLE</Badge> : null}
        </div>
      </div>

      <p className="mod-card__description">
        {mod.description || `Zainstalowany plik: ${mod.fileName}`}
      </p>

      <div className="mod-card__meta">
        <span>v{mod.installedVersion || mod.version}</span>
        <span>
          <FileArchive size={12} />
          {formatBytes(mod.fileSize)}
        </span>
        <span>
          <Calendar size={12} />
          {formatDate(mod.installedAt)}
        </span>
      </div>

      {removalBlocked ? (
        <p className="mod-card__reason">
          <AlertTriangle size={13} />
          {mod.required
            ? "This component is required by Private Client."
            : `Required by: `}
        </p>
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
          DETAILS
          <ChevronDown
            size={14}
            className={expanded ? "is-rotated" : ""}
            aria-hidden="true"
          />
        </button>
        <div className="installed-card__footer-actions">
          {mod.updateAvailable ? (
            <Button
              variant="secondary"
              size="sm"
              busy={busy}
              icon={<RefreshCcw size={14} />}
              onClick={() => {
                onUpdate(mod);
              }}
            >
              UPDATE
            </Button>
          ) : null}
          <Button
            variant="danger"
            size="sm"
            busy={busy}
            disabled={removalBlocked}
            icon={<Trash2 size={14} />}
            onClick={() => {
              onRemove(mod);
            }}
          >
            REMOVE
          </Button>
        </div>
      </div>

      {expanded ? (
        <motion.dl
          className="mod-card__details"
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
        >
          <div>
            <dt>File</dt>
            <dd title={mod.fileName}>{mod.fileName}</dd>
          </div>
          <div>
            <dt>Size</dt>
            <dd>{formatBytes(mod.fileSize)}</dd>
          </div>
          <div>
            <dt>License</dt>
            <dd>{mod.license}</dd>
          </div>
          <div className="mod-card__details--full">
            <dt>
              <Hash size={11} /> SHA-512
            </dt>
            <dd className="sha-text" title={mod.sha512}>
              {mod.sha512}
            </dd>
          </div>
        </motion.dl>
      ) : null}
    </motion.article>
  );
}
