import { useState } from "react";
import { Cpu, Download, RefreshCcw, Save, Zap } from "lucide-react";
import { Modal } from "../../components/common/Modal";
import { Button } from "../../components/common/Button";
import { useLauncherStore } from "../../stores/useLauncherStore";
import { useUiStore } from "../../stores/useUiStore";
import { launcherSettingsSchema, type LauncherSettings } from "../../types/contracts";

export function SettingsModal() {
  const settings = useLauncherStore((state) => state.snapshot?.settings);
  const saveSettings = useLauncherStore((state) => state.saveSettings);
  const appVersion = useLauncherStore((state) => state.snapshot?.appVersion ?? "—");
  const updateStatus = useLauncherStore((state) => state.update);
  const updateChecking = useLauncherStore((state) => state.updateChecking);
  const updateInstalling = useLauncherStore((state) => state.updateInstalling);
  const checkForUpdate = useLauncherStore((state) => state.checkForUpdate);
  const installUpdate = useLauncherStore((state) => state.installUpdate);
  const closeModal = useUiStore((state) => state.closeModal);
  // `edits` holds only what the user changed. The snapshot arrives
  // asynchronously (it awaits Java detection), so the modal can mount before
  // settings exist; deriving the draft during render lets it pick them up
  // without an effect and without discarding edits already in progress.
  const [edits, setEdits] = useState<LauncherSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const draft = edits ?? settings ?? null;
  const validation = draft ? launcherSettingsSchema.safeParse(draft) : null;

  const update = <TKey extends keyof LauncherSettings>(
    key: TKey,
    value: LauncherSettings[TKey],
  ) => {
    setEdits((current) => {
      const base = current ?? settings;
      return base ? { ...base, [key]: value } : current;
    });
  };

  // A single slider drives the maximum heap; the minimum follows it down so the
  // pair can never end up inverted and rejected by the schema.
  const setAllocatedMemory = (maxMb: number) => {
    setEdits((current) => {
      const base = current ?? settings;
      if (!base) {
        return current;
      }
      return { ...base, memoryMaxMb: maxMb, memoryMinMb: Math.min(base.memoryMinMb, maxMb) };
    });
  };

  return (
    <Modal
      title="Ustawienia techniczne"
      eyebrow="LOKALNA KONFIGURACJA"
      description="Parametry instancji są walidowane przez backend i zapisywane atomowo na tym urządzeniu."
      onClose={closeModal}
      footer={
        <>
          <Button variant="ghost" onClick={closeModal}>
            ANULUJ
          </Button>
          <Button
            variant="primary"
            busy={saving}
            disabled={!validation?.success}
            icon={<Save size={16} />}
            onClick={() => {
              if (!draft || !validation?.success) {
                return;
              }
              setSaving(true);
              void saveSettings(draft).then((saved) => {
                setSaving(false);
                if (saved) {
                  closeModal();
                }
              });
            }}
          >
            ZAPISZ
          </Button>
        </>
      }
    >
      {draft ? (
        <div className="settings-form">
          <label className="field field--full">
            <span className="field__label">
              <Cpu size={14} />
              Ścieżka do Java 8
            </span>
            <input
              value={draft.javaPath ?? ""}
              placeholder="Wykryj automatycznie"
              onChange={(event) => {
                update("javaPath", event.target.value.trim() || null);
              }}
            />
            <small>Backend potwierdzi wersję, architekturę i możliwość uruchomienia.</small>
          </label>

          <label className="field field--full">
            <span className="field__label">
              <Zap size={14} />
              Maksymalny RAM
              <strong className="field__value">{draft.memoryMaxMb} MB</strong>
            </span>
            <div className="field__slider">
              <input
                type="range"
                min={1024}
                max={16_384}
                step={256}
                value={draft.memoryMaxMb}
                aria-label="Maksymalny RAM w MB"
                onChange={(event) => {
                  setAllocatedMemory(Number(event.target.value));
                }}
              />
              <div className="field__slider-scale">
                <span>1024 MB</span>
                <span>16384 MB</span>
              </div>
            </div>
            <small>
              Minimum pozostaje na {draft.memoryMinMb} MB, maksimum wyznacza suwak.
            </small>
          </label>

          {validation && !validation.success ? (
            <p className="field-error">
              {validation.error.issues[0]?.message ?? "Sprawdź ustawienia pamięci."}
            </p>
          ) : null}

          <label className="toggle-row">
            <span className="toggle-row__icon">
              <RefreshCcw size={17} />
            </span>
            <span>
              <strong>Automatyczne sprawdzanie aktualizacji</strong>
              <small>
                Przy starcie launchera pyta podpisany kanał aktualizacji o nowszą wersję.
                Nic nie instaluje się bez Twojej zgody.
              </small>
            </span>
            <input
              type="checkbox"
              checked={draft.autoUpdateChecks}
              aria-label="Automatyczne sprawdzanie aktualizacji"
              onChange={(event) => {
                update("autoUpdateChecks", event.target.checked);
              }}
            />
          </label>

          <div className="field field--short">
            <span className="field__label">Aktualizacja launchera</span>
            <div className="field__readonly">
              <strong>
                {updateStatus?.available
                  ? `Dostępna wersja ${updateStatus?.availableVersion}`
                  : `Wersja ${appVersion}`}
              </strong>
              <Button
                variant={updateStatus?.available ? "primary" : "ghost"}
                busy={updateStatus?.available ? updateInstalling : updateChecking}
                icon={<Download size={16} />}
                onClick={() => {
                  if (updateStatus?.available) {
                    void installUpdate();
                  } else {
                    void checkForUpdate();
                  }
                }}
              >
                {updateStatus?.available ? "ZAINSTALUJ" : "SPRAWDŹ"}
              </Button>
            </div>
          </div>

          <label className="toggle-row">
            <span className="toggle-row__icon">
              <Zap size={17} />
            </span>
            <span>
              <strong>Ogranicz animacje</strong>
              <small>Wyłącza parallax, autoobrót modelu i złożone przejścia.</small>
            </span>
            <input
              type="checkbox"
              checked={draft.reducedMotion}
              onChange={(event) => {
                update("reducedMotion", event.target.checked);
              }}
            />
          </label>

          <div className="field field--short">
            <span className="field__label">Równoległe pobierania</span>
            <div className="field__readonly">
              <strong>{draft.downloadConcurrency}</strong>
              <span>Stała wartość backendu</span>
            </div>
          </div>
        </div>
      ) : (
        <div className="skeleton" style={{ height: 360 }} />
      )}
    </Modal>
  );
}
