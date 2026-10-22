interface StatusBadgeProps {
  label: string;
  tone?: "ready" | "idle";
}

export function StatusBadge({ label, tone = "idle" }: StatusBadgeProps) {
  const ready = tone === "ready";
  return (
    <span className="inline-flex items-center gap-2 rounded-full border border-white/8 bg-black/20 px-3 py-1.5 text-xs text-zinc-400">
      <span className={`size-1.5 rounded-full ${ready ? "bg-emerald-400" : "bg-amber-400"}`} />
      {label}
    </span>
  );
}
