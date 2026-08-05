import { Box } from "lucide-react";
import { initials } from "../../lib/format";

interface ModIconProps {
  name: string;
  iconUrl: string | null;
  size?: "sm" | "md";
}

export function ModIcon({ name, iconUrl, size = "md" }: ModIconProps) {
  return (
    <span className={`mod-icon mod-icon--${size}`}>
      {iconUrl ? (
        <img src={iconUrl} alt="" loading="lazy" referrerPolicy="no-referrer" />
      ) : (
        <>
          <Box className="mod-icon__box" size={size === "md" ? 22 : 16} />
          <span>{initials(name)}</span>
        </>
      )}
    </span>
  );
}
