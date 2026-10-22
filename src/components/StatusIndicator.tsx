export type StatusTone = "online" | "offline" | "loading" | "error";

interface StatusIndicatorProps {
  label: string;
  tone: StatusTone;
}

const toneStyles: Record<StatusTone, string> = {
  online: "bg-emerald-400",
  offline: "bg-zinc-500",
  loading: "bg-amber-300",
  error: "bg-rose-400",
};

export function StatusIndicator({ label, tone }: StatusIndicatorProps) {
  return (
    <span className="inline-flex min-h-8 items-center gap-2 rounded-full border border-white/[0.08] bg-white/[0.025] px-3 text-xs text-zinc-400">
      <span className={`size-1.5 rounded-full ${toneStyles[tone]}`} aria-hidden="true" />
      {label}
    </span>
  );
}
