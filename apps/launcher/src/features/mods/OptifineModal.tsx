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
      title="Install Private Pack"
      eyebrow="CURATED 1.8.9 MOD PACK"
      description="Private Pack installs OptiFine, the original PvP mods and a set of compatible Forge 1.8.9 optimizations. Components are downloaded separately and checked against pinned hashes, but the launcher presents them as one managed pack."
      onClose={close}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            CANCEL
          </Button>
          <Button
            variant="primary"
            busy={busy}
            icon={<Download size={16} />}
            onClick={() => {
              void downloadOptifine();
            }}
          >
            DOWNLOAD AND INSTALL PRIVATE PACK
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
            <strong>1. Click Import</strong>
          </div>
        </article>
        <article>
          <span>
            <PackageCheck size={18} />
          </span>
          <div>
            <strong>2. The required mod pack installs itself</strong>
          </div>
        </article>
        <article>
          <span>
            <Sparkles size={18} />
          </span>
          <div>
            <strong>3. Enjoy the feel of a top-tier client</strong>
          </div>
        </article>
      </div>
      <p className="optifine-notice">
        <ShieldCheck size={15} />
        Private Pack is downloaded from official, verified Modrinth and GitHub
        sources.
      </p>
    </Modal>
  );
}
