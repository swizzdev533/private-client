import { useEffect } from "react";
import { motion, useMotionTemplate, useMotionValue, useSpring } from "framer-motion";

interface BackgroundSceneProps {
  reducedMotion: boolean;
}

export function BackgroundScene({ reducedMotion }: BackgroundSceneProps) {
  const pointerX = useMotionValue(
    typeof window === "undefined" ? 0 : window.innerWidth / 2,
  );
  const pointerY = useMotionValue(
    typeof window === "undefined" ? 0 : window.innerHeight / 2,
  );
  const smoothX = useSpring(pointerX, { stiffness: 90, damping: 24 });
  const smoothY = useSpring(pointerY, { stiffness: 90, damping: 24 });
  const glow = useMotionTemplate`radial-gradient(520px circle at ${smoothX}px ${smoothY}px, rgba(255,255,255,.11), rgba(255,255,255,.025) 34%, transparent 69%)`;

  useEffect(() => {
    if (reducedMotion) {
      return undefined;
    }

    let frame = 0;
    const handlePointerMove = (event: PointerEvent) => {
      window.cancelAnimationFrame(frame);
      frame = window.requestAnimationFrame(() => {
        pointerX.set(event.clientX);
        pointerY.set(event.clientY);
      });
    };

    window.addEventListener("pointermove", handlePointerMove, { passive: true });
    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("pointermove", handlePointerMove);
    };
  }, [pointerX, pointerY, reducedMotion]);

  return (
    <div className="background-scene" aria-hidden="true">
      <div className="background-scene__image" />
      <div className="background-scene__vignette" />
      <div className="background-scene__noise" />
      <motion.div
        className="background-scene__pointer-glow"
        style={reducedMotion ? {} : { backgroundImage: glow }}
      />
      <div className="background-scene__grid" />
      <div className="background-scene__scanline" />
    </div>
  );
}
