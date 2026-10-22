interface ToggleSwitchProps {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

export function ToggleSwitch({
  label,
  description,
  checked,
  disabled = false,
  onChange,
}: ToggleSwitchProps) {
  return (
    <div className="flex items-start justify-between gap-5">
      <div>
        <p className={`text-sm font-medium ${disabled ? "text-zinc-600" : "text-zinc-300"}`}>
          {label}
        </p>
        {description ? <p className="mt-1 text-xs leading-5 text-zinc-600">{description}</p> : null}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`relative mt-0.5 h-6 w-11 shrink-0 rounded-full border transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-300 disabled:cursor-not-allowed disabled:opacity-40 ${
          checked ? "border-cyan-300/60 bg-cyan-300/25" : "border-white/10 bg-white/[0.04]"
        }`}
      >
        <span
          className={`absolute top-1/2 size-4 -translate-y-1/2 rounded-full transition-transform ${
            checked ? "translate-x-5 bg-cyan-200" : "translate-x-1 bg-zinc-500"
          }`}
        />
      </button>
    </div>
  );
}
