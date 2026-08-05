interface ProgressBarProps {
  value: number | null;
  label?: string;
  compact?: boolean;
}

export function ProgressBar({ value, label, compact = false }: ProgressBarProps) {
  const normalized = Math.min(100, Math.max(0, value ?? 24));

  return (
    <div className={`progress ${compact ? "progress--compact" : ""}`}>
      {label ? (
        <div className="progress__meta">
          <span>{label}</span>
          {value === null ? <span>Pracuję…</span> : <span>{Math.round(value)}%</span>}
        </div>
      ) : null}
      <div
        className={`progress__track ${value === null ? "is-indeterminate" : ""}`}
        role="progressbar"
        aria-label={label ?? "Postęp operacji"}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={value ?? undefined}
      >
        <span
          className="progress__fill"
          style={{ width: value === null ? "32%" : `${normalized}%` }}
        />
      </div>
    </div>
  );
}
