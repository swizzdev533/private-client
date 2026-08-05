import type { ButtonHTMLAttributes, ReactNode } from "react";
import { LoaderCircle } from "lucide-react";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md" | "lg" | "icon";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  busy?: boolean;
  icon?: ReactNode;
}

export function Button({
  variant = "secondary",
  size = "md",
  busy = false,
  icon,
  className = "",
  children,
  disabled,
  type = "button",
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={`button button--${variant} button--${size} ${className}`}
      disabled={disabled || busy}
      aria-busy={busy}
      {...props}
    >
      <span className="button__shine" aria-hidden="true" />
      {busy ? (
        <LoaderCircle className="button__spinner" size={17} aria-hidden="true" />
      ) : (
        icon
      )}
      {children ? <span className="button__label">{children}</span> : null}
    </button>
  );
}
