export function SoundVolumeSlider({
  value,
  disabled,
  onChange,
}: {
  value: number;
  disabled?: boolean;
  onChange: (value: number) => void;
}) {
  return (
    <label className="block text-xs text-zinc-500">
      Volume <span className="float-right font-mono text-zinc-300">{Math.round(value)}%</span>
      <input
        aria-label="Sound volume"
        className="mt-2 w-full accent-cyan-300"
        type="range"
        min="0"
        max="100"
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </label>
  );
}
