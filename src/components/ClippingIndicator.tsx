interface ClippingIndicatorProps {
  clipping: boolean;
}

export function ClippingIndicator({ clipping }: ClippingIndicatorProps) {
  return (
    <span
      role="status"
      aria-live="polite"
      className={`inline-flex min-h-7 items-center rounded-full border px-2.5 text-[10px] font-medium uppercase tracking-wide ${
        clipping
          ? "border-rose-300/30 bg-rose-300/10 text-rose-200"
          : "border-white/[0.07] text-zinc-600"
      }`}
    >
      {clipping ? "Clipping" : "Headroom OK"}
    </span>
  );
}
