import type { ReactNode } from "react";

export type BadgeTone = "neutral" | "bright" | "success" | "warning" | "danger";

interface BadgeProps {
  children: ReactNode;
  tone?: BadgeTone;
  dot?: boolean;
  className?: string;
}

export function Badge({
  children,
  tone = "neutral",
  dot = false,
  className = "",
}: BadgeProps) {
  return (
    <span className={`badge badge--${tone} ${className}`}>
      {dot ? <span className="badge__dot" aria-hidden="true" /> : null}
      {children}
    </span>
  );
}
