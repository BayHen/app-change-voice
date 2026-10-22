import { Check, LockSimple } from "@phosphor-icons/react";
import type { VoicePresetSummary } from "../types/preset";

interface EffectCardProps {
  preset: VoicePresetSummary;
  selected: boolean;
  onSelect: (preset: VoicePresetSummary) => void;
}

export function EffectCard({ preset, selected, onSelect }: EffectCardProps) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={() => onSelect(preset)}
      className={`group min-h-40 w-full rounded-2xl border p-4 text-left transition-colors active:translate-y-px ${
        selected
          ? "border-cyan-300/45 bg-cyan-300/[0.07]"
          : "border-white/[0.08] bg-[#101318] hover:border-white/[0.16] hover:bg-[#12161b]"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <span className="text-[11px] font-medium text-zinc-600">{preset.category}</span>
        {selected ? (
          <span className="grid size-6 place-items-center rounded-full bg-cyan-300 text-[#071014]">
            <Check size={13} weight="bold" aria-hidden="true" />
          </span>
        ) : !preset.available ? (
          <LockSimple size={15} className="text-zinc-700" aria-hidden="true" />
        ) : null}
      </div>
      <h3 className="mt-7 text-sm font-semibold text-zinc-200">{preset.name}</h3>
      <p className="mt-2 line-clamp-2 text-xs leading-5 text-zinc-600">{preset.description}</p>
    </button>
  );
}
