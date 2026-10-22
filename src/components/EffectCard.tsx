import { Check } from "@phosphor-icons/react";
import type { VoicePresetSummary } from "../types/preset";

interface EffectCardProps {
  preset: VoicePresetSummary;
  selected: boolean;
  active: boolean;
  onSelect: (preset: VoicePresetSummary) => void;
}

export function EffectCard({ preset, selected, active, onSelect }: EffectCardProps) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={() => onSelect(preset)}
      className={`group min-h-40 w-full rounded-2xl border p-4 text-left transition-colors active:translate-y-px ${
        active
          ? "border-cyan-300/45 bg-cyan-300/[0.07]"
          : selected
            ? "border-white/20 bg-white/[0.04]"
            : "border-white/[0.08] bg-[#101318] hover:border-white/[0.16] hover:bg-[#12161b]"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <span className="text-[11px] font-medium text-zinc-600">{preset.category}</span>
        {active ? (
          <span className="grid size-6 place-items-center rounded-full bg-cyan-300 text-[#071014]">
            <Check size={13} weight="bold" aria-hidden="true" />
          </span>
        ) : null}
      </div>
      <h3 className="mt-7 text-sm font-semibold text-zinc-200">{preset.name}</h3>
      <p className="mt-2 line-clamp-2 text-xs leading-5 text-zinc-600">{preset.description}</p>
      {active ? <p className="mt-3 text-[11px] font-medium text-cyan-300">Active preset</p> : null}
    </button>
  );
}
