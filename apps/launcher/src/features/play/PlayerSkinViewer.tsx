import { useEffect, useMemo, useRef, useState } from "react";
import { IdleAnimation, SkinViewer } from "skinview3d";
import { Rotate3D } from "lucide-react";
import { localAssetUrl } from "../../lib/tauriBridge";
import type { PlayerProfile } from "../../types/contracts";

interface PlayerSkinViewerProps {
  profile: PlayerProfile | null;
  reducedMotion: boolean;
}

function createFallbackSkin(): string {
  if (import.meta.env.MODE === "test") {
    return "";
  }
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const context = canvas.getContext("2d");
  if (!context) {
    return "";
  }

  context.imageSmoothingEnabled = false;
  context.fillStyle = "#000000";
  context.fillRect(0, 0, 64, 64);
  return canvas.toDataURL("image/png");
}

export function PlayerSkinViewer({ profile, reducedMotion }: PlayerSkinViewerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const viewerRef = useRef<SkinViewer | null>(null);
  const [viewerReady, setViewerReady] = useState(false);
  const skinSource = localAssetUrl(profile?.skinPath ?? null);
  const fallbackSkin = useMemo(
    () => (typeof document === "undefined" ? "" : createFallbackSkin()),
    [],
  );

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || import.meta.env.MODE === "test") {
      return undefined;
    }

    try {
      const viewer = new SkinViewer({
        canvas,
        width: 380,
        height: 510,
      });
      viewerRef.current = viewer;
      viewer.zoom = 0.78;
      viewer.fov = 42;
      viewer.autoRotate = !reducedMotion;
      viewer.autoRotateSpeed = 0.38;
      viewer.controls.enablePan = false;
      viewer.controls.enableZoom = false;
      const animation = new IdleAnimation();
      viewer.animation = animation;
      animation.speed = reducedMotion ? 0 : 0.55;
      void viewer
        .loadSkin(skinSource ?? fallbackSkin, {
          model: profile?.skinModel === "slim" ? "slim" : "default",
        })
        .then(() => {
          setViewerReady(true);
        })
        .catch(() => {
          if (skinSource) {
            void viewer.loadSkin(fallbackSkin, { model: "default" }).then(() => {
              setViewerReady(true);
            });
          }
        });

      const resize = new ResizeObserver(([entry]) => {
        if (!entry) {
          return;
        }
        const width = Math.min(420, Math.max(260, entry.contentRect.width));
        viewer.width = width;
        viewer.height = Math.min(540, Math.max(390, entry.contentRect.height));
      });
      resize.observe(canvas.parentElement ?? canvas);

      const handleVisibility = () => {
        const active = !document.hidden && !reducedMotion;
        viewer.autoRotate = active;
        animation.speed = active ? 0.55 : 0;
      };
      document.addEventListener("visibilitychange", handleVisibility);

      return () => {
        document.removeEventListener("visibilitychange", handleVisibility);
        resize.disconnect();
        viewer.dispose();
        viewerRef.current = null;
      };
    } catch {
      return undefined;
    }
  }, [fallbackSkin, profile?.skinModel, reducedMotion, skinSource]);

  return (
    <div className="player-viewer">
      <div className="player-viewer__halo" aria-hidden="true" />
      <div
        className={`player-viewer__fallback ${viewerReady ? "is-hidden" : ""}`}
        aria-hidden="true"
      >
        <div className="block-player">
          <span className="block-player__head" />
          <span className="block-player__body" />
          <span className="block-player__arm block-player__arm--left" />
          <span className="block-player__arm block-player__arm--right" />
          <span className="block-player__leg block-player__leg--left" />
          <span className="block-player__leg block-player__leg--right" />
        </div>
      </div>
      <canvas
        ref={canvasRef}
        className={`player-viewer__canvas ${viewerReady ? "is-ready" : ""}`}
        aria-label={
          profile ? `Player skin model for ` : "Placeholder player model"
        }
      />
      <div className="player-viewer__hint">
        <Rotate3D size={13} aria-hidden="true" />
        <span>Drag to rotate</span>
      </div>
    </div>
  );
}
