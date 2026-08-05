import {
  AlertTriangle,
  ArrowDownToLine,
  Boxes,
  CheckCircle2,
  Database,
  FileArchive,
  ShieldCheck,
} from "lucide-react";
import { Modal } from "../../components/common/Modal";
import { Button } from "../../components/common/Button";
import { Badge } from "../../components/common/Badge";
import { ProgressBar } from "../../components/common/ProgressBar";
import { formatBytes } from "../../lib/format";
import { useModsStore } from "../../stores/useModsStore";

export function InstallPlanModal() {
  const plan = useModsStore((state) => state.installPlan);
  const selectedMod = useModsStore((state) => state.selectedMod);
  const mutatingProjectId = useModsStore((state) => state.mutatingProjectId);
  const operationProgress = useModsStore((state) => state.operationProgress);
  const close = useModsStore((state) => state.closeInstallPlan);
  const confirm = useModsStore((state) => state.confirmInstall);
  const busy = selectedMod?.projectId === mutatingProjectId;

  return (
    <Modal
      title={selectedMod ? `Zainstaluj ${selectedMod.name}` : "Plan instalacji"}
      eyebrow="TRANSAKCJA ATOMOWA"
      description="Backend przypina konkretną wersję, weryfikuje źródło, zależności, strukturę JAR i SHA-512 przed zmianą instancji."
      onClose={close}
      width="lg"
      footer={
        <>
          <Button variant="ghost" onClick={close} disabled={busy}>
            ANULUJ
          </Button>
          <Button
            variant="primary"
            icon={<ArrowDownToLine size={16} />}
            busy={busy}
            disabled={!plan}
            onClick={() => {
              void confirm();
            }}
          >
            ZATWIERDŹ I ZAINSTALUJ
          </Button>
        </>
      }
    >
      {!plan ? (
        <div className="install-plan-skeleton">
          <div className="skeleton" />
          <div className="skeleton" />
          <div className="skeleton" />
        </div>
      ) : (
        <div className="install-plan">
          <section className="install-plan__hero">
            <span className="install-plan__hero-icon">
              <FileArchive size={24} />
            </span>
            <div>
              <span>WYBRANE WYDANIE</span>
              <h3>{plan.requestedMod.name}</h3>
              <p>{plan.requestedMod.version} · Minecraft 1.8.9 · Forge</p>
            </div>
            <Badge tone="success">
              <ShieldCheck size={11} />
              GOTOWY PLAN
            </Badge>
          </section>

          <dl className="install-plan__facts">
            <div>
              <dt>
                <Database size={13} />
                Rozmiar pliku
              </dt>
              <dd>{formatBytes(plan.requestedMod.fileSize)}</dd>
            </div>
            <div>
              <dt>
                <Boxes size={13} />
                Wymagane zależności
              </dt>
              <dd>{plan.dependencies.length}</dd>
            </div>
            <div>
              <dt>
                <ArrowDownToLine size={13} />
                Miejsce transakcji
              </dt>
              <dd>{formatBytes(plan.expectedDiskUsage)}</dd>
            </div>
          </dl>

          {plan.dependencies.length > 0 ? (
            <section className="install-plan__dependencies">
              <span className="eyebrow">ZALEŻNOŚCI</span>
              {plan.dependencies.map((dependency) => (
                <article key={dependency.versionId}>
                  <CheckCircle2 size={15} />
                  <div>
                    <strong>{dependency.name}</strong>
                    <span>
                      {dependency.version} · {formatBytes(dependency.fileSize)}
                    </span>
                  </div>
                  <Badge tone="neutral">REQUIRED</Badge>
                </article>
              ))}
            </section>
          ) : null}

          {plan.filesToReplace.length > 0 ? (
            <p className="install-plan__warning">
              <AlertTriangle size={15} />
              Poprzedni plik zostanie zachowany do czasu zatwierdzenia transakcji.
            </p>
          ) : null}

          {plan.warnings.map((warning) => (
            <p className="install-plan__warning" key={warning}>
              <AlertTriangle size={15} />
              {warning}
            </p>
          ))}

          {operationProgress && operationProgress.targetId === selectedMod?.projectId ? (
            <div className="install-plan__progress">
              <ProgressBar
                value={operationProgress.progress}
                label={operationProgress.message}
              />
              <code>{operationProgress.phase}</code>
            </div>
          ) : null}
        </div>
      )}
    </Modal>
  );
}
