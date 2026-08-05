import {
  Download,
  MousePointerClick,
  PackageCheck,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { Modal } from "../../components/common/Modal";
import { Button } from "../../components/common/Button";
import { useModsStore } from "../../stores/useModsStore";
import { useUiStore } from "../../stores/useUiStore";

export function OptifineModal() {
  const close = useUiStore((state) => state.closeModal);
  const downloadOptifine = useModsStore((state) => state.downloadOptifine);
  const busy = useModsStore(
    (state) =>
      state.mutatingProjectId === "local-private-pack" ||
      state.mutatingProjectId === "local-optifine",
  );

  return (
    <Modal
      title="Instalacja Private Pack"
      eyebrow="DEDYKOWANY PAKIET MODÓW 1.8.9"
      description="Private Pack instaluje OptiFine, oryginalne mody PvP i zestaw zgodnych optymalizacji Forge 1.8.9. Składniki są pobierane osobno i sprawdzane przypiętymi hashami, ale launcher prezentuje je jako jeden zarządzany pakiet."
      onClose={close}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            ANULUJ
          </Button>
          <Button
            variant="primary"
            busy={busy}
            icon={<Download size={16} />}
            onClick={() => {
              void downloadOptifine();
            }}
          >
            POBIERZ I ZAINSTALUJ PRIVATE PACK
          </Button>
        </>
      }
    >
      <div className="optifine-flow">
        <article>
          <span>
            <MousePointerClick size={18} />
          </span>
          <div>
            <strong>1. Kliknij Importuj</strong>
          </div>
        </article>
        <article>
          <span>
            <PackageCheck size={18} />
          </span>
          <div>
            <strong>2. Paczka niezbędnych modów zainstaluje się sama</strong>
          </div>
        </article>
        <article>
          <span>
            <Sparkles size={18} />
          </span>
          <div>
            <strong>3. Ciesz się feelingiem topowych clientów</strong>
          </div>
        </article>
      </div>
      <p className="optifine-notice">
        <ShieldCheck size={15} />
        Pakiet Private Pack jest pobierany z oficjalnych i zweryfikowanych źródeł Modrinth /
        GitHub.
      </p>
    </Modal>
  );
}
