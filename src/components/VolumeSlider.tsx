interface VolumeSliderProps {
  id: string;
  label: string;
  value: number;
  min?: number;
  max?: number;
  unit?: string;
  disabled?: boolean;
  onChange: (value: number) => void;
  onCommit?: (value: number) => void;
}

export function VolumeSlider({
  id,
  label,
  value,
  min = 0,
  max = 100,
  unit = "%",
  disabled = false,
  onChange,
  onCommit,
}: VolumeSliderProps) {
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-4">
        <label htmlFor={id} className="text-xs font-medium text-zinc-400">
          {label}
        </label>
        <output htmlFor={id} className="min-w-12 text-right font-mono text-xs text-zinc-500">
          {value}
          {unit}
        </output>
      </div>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(Number(event.target.value))}
        onPointerUp={(event) => onCommit?.(Number(event.currentTarget.value))}
        onKeyUp={(event) => onCommit?.(Number(event.currentTarget.value))}
        className="audio-range w-full disabled:cursor-not-allowed disabled:opacity-40"
      />
    </div>
  );
}
