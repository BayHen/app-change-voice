import { useState } from "react";

interface ShortcutRecorderProps {
  label: string;
  value: string | null;
  disabled?: boolean;
  duplicate?: boolean;
  onChange: (value: string | null) => void;
}

const modifiers = new Set(["Control", "Alt", "Shift", "Meta"]);

export function ShortcutRecorder({
  label,
  value,
  disabled,
  duplicate,
  onChange,
}: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(false);
  return (
    <div className="grid gap-3 rounded-xl border border-white/[0.07] bg-white/[0.015] p-3 sm:grid-cols-[1fr_210px_auto] sm:items-center">
      <span className="text-xs font-medium text-zinc-300">{label}</span>
      <button
        type="button"
        disabled={disabled}
        aria-label={`Assign shortcut for ${label}`}
        onClick={() => setRecording(true)}
        onKeyDown={(event) => {
          if (!recording) return;
          event.preventDefault();
          event.stopPropagation();
          if (event.key === "Escape") {
            setRecording(false);
            return;
          }
          if (modifiers.has(event.key)) return;
          const parts = [
            event.ctrlKey && "Ctrl",
            event.altKey && "Alt",
            event.shiftKey && "Shift",
            event.metaKey && "Super",
          ].filter(Boolean);
          if (parts.length === 0) return;
          const key =
            event.key.length === 1 ? event.key.toUpperCase() : event.key.replace("Arrow", "");
          onChange([...parts, key].join("+"));
          setRecording(false);
        }}
        className={`h-9 rounded-lg border px-3 text-left font-mono text-xs ${recording ? "border-cyan-300/50 bg-cyan-300/[0.06] text-cyan-200" : duplicate ? "border-rose-300/40 text-rose-200" : "border-white/[0.09] text-zinc-400"}`}
      >
        {recording ? "Press shortcut…" : (value ?? "Not assigned")}
      </button>
      <button
        type="button"
        disabled={disabled || !value}
        onClick={() => onChange(null)}
        className="h-9 rounded-lg border border-white/[0.08] px-3 text-xs text-zinc-500 hover:text-zinc-200 disabled:opacity-40"
      >
        Remove
      </button>
    </div>
  );
}
