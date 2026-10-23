export function HotkeyAssignment({
  value,
  onChange,
}: {
  value: string | null;
  onChange: (value: string | null) => void;
}) {
  return (
    <label className="block text-xs text-zinc-500">
      Hotkey
      <input
        aria-label="Sound hotkey"
        className="mt-2 w-full rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-sm text-zinc-200 outline-none focus:border-cyan-300/50"
        value={value ?? ""}
        placeholder="Ctrl+Alt+1"
        onChange={(e) => onChange(e.target.value || null)}
      />
    </label>
  );
}
