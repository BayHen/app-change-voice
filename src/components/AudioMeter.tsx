interface AudioMeterProps {
  label: string;
  level: number;
  disabled?: boolean;
}

const segments = Array.from({ length: 18 }, (_, index) => index + 1);

export function AudioMeter({ label, level, disabled = false }: AudioMeterProps) {
  const safeLevel = Math.min(100, Math.max(0, level));
  const activeSegments = Math.round((safeLevel / 100) * segments.length);

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <span className="text-xs font-medium text-zinc-400">{label}</span>
        <span className="font-mono text-[11px] text-zinc-600">
          {disabled ? "No signal" : `${safeLevel}%`}
        </span>
      </div>
      <div
        role="meter"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={safeLevel}
        className="grid h-8 grid-cols-[repeat(18,minmax(2px,1fr))] items-end gap-1"
      >
        {segments.map((segment) => {
          const active = !disabled && segment <= activeSegments;
          return (
            <span
              key={segment}
              aria-hidden="true"
              className={`rounded-sm ${active ? "bg-cyan-300" : "bg-white/[0.06]"}`}
              style={{ height: `${36 + segment * 3.4}%` }}
            />
          );
        })}
      </div>
    </div>
  );
}
