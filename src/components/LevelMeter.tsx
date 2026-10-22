interface LevelMeterProps {
  label: string;
  peakDbfs: number;
  rmsDbfs: number;
  disabled?: boolean;
}

const segments = Array.from({ length: 20 }, (_, index) => index);

function normalized(dbfs: number) {
  return Math.min(1, Math.max(0, (dbfs + 60) / 60));
}

export function LevelMeter({ label, peakDbfs, rmsDbfs, disabled = false }: LevelMeterProps) {
  const active = Math.round(normalized(peakDbfs) * segments.length);
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <span className="text-xs font-medium text-zinc-400">{label}</span>
        <span className="font-mono text-[11px] text-zinc-500">
          {disabled ? "No signal" : `${rmsDbfs.toFixed(1)} dBFS RMS · ${peakDbfs.toFixed(1)} peak`}
        </span>
      </div>
      <div
        role="meter"
        aria-label={label}
        aria-valuemin={-60}
        aria-valuemax={0}
        aria-valuenow={disabled ? -60 : Math.max(-60, peakDbfs)}
        className="grid h-8 grid-cols-[repeat(20,minmax(2px,1fr))] items-end gap-1"
      >
        {segments.map((segment) => (
          <span
            key={segment}
            aria-hidden="true"
            className={`rounded-sm ${
              !disabled && segment < active
                ? segment >= 18
                  ? "bg-rose-400"
                  : segment >= 15
                    ? "bg-amber-300"
                    : "bg-cyan-300"
                : "bg-white/[0.06]"
            }`}
            style={{ height: `${36 + segment * 3.1}%` }}
          />
        ))}
      </div>
    </div>
  );
}
