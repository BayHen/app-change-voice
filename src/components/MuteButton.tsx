import { SpeakerHigh, SpeakerSlash } from "@phosphor-icons/react";

interface MuteButtonProps {
  label: string;
  muted: boolean;
  disabled?: boolean;
  onChange: (muted: boolean) => void;
}

export function MuteButton({ label, muted, disabled = false, onChange }: MuteButtonProps) {
  const Icon = muted ? SpeakerSlash : SpeakerHigh;
  return (
    <button
      type="button"
      aria-label={`${muted ? "Unmute" : "Mute"} ${label}`}
      aria-pressed={muted}
      disabled={disabled}
      onClick={() => onChange(!muted)}
      className={`inline-flex h-9 items-center gap-2 rounded-lg border px-3 text-xs transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${
        muted
          ? "border-rose-300/25 bg-rose-300/[0.08] text-rose-200"
          : "border-white/[0.09] text-zinc-400 hover:text-zinc-200"
      }`}
    >
      <Icon size={15} aria-hidden="true" />
      {muted ? "Muted" : "Mute"}
    </button>
  );
}
