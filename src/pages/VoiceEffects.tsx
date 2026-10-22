import { MagnifyingGlass, SlidersHorizontal } from "@phosphor-icons/react";
import { useMemo, useState } from "react";
import { AppHeader } from "../components/AppHeader";
import { EffectCard } from "../components/EffectCard";
import { presetCategories, voicePresets } from "../data/presets";
import type { PresetCategory, VoicePresetSummary } from "../types/preset";

interface VoiceEffectsProps {
  onPresetSelected: (preset: VoicePresetSummary) => void;
}

export function VoiceEffects({ onPresetSelected }: VoiceEffectsProps) {
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<"All" | PresetCategory>("All");
  const [selectedPreset, setSelectedPreset] = useState(voicePresets[0]);

  const filteredPresets = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return voicePresets.filter((preset) => {
      const categoryMatches = category === "All" || preset.category === category;
      const queryMatches =
        !normalizedQuery ||
        preset.name.toLowerCase().includes(normalizedQuery) ||
        preset.description.toLowerCase().includes(normalizedQuery);
      return categoryMatches && queryMatches;
    });
  }, [category, query]);

  function selectPreset(preset: VoicePresetSummary) {
    setSelectedPreset(preset);
    onPresetSelected(preset);
  }

  return (
    <div className="mx-auto max-w-[1180px]">
      <AppHeader
        title="Voice Effects"
        description="Browse the preset catalog. Selection works locally, while audio application remains disabled."
      />

      <div className="mt-6 grid gap-5 xl:grid-cols-[1fr_300px]">
        <section className="min-w-0">
          <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div className="relative w-full lg:max-w-sm">
              <label htmlFor="preset-search" className="sr-only">
                Search presets
              </label>
              <MagnifyingGlass
                size={17}
                className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-zinc-600"
                aria-hidden="true"
              />
              <input
                id="preset-search"
                type="search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Search presets"
                className="h-11 w-full rounded-xl border border-white/[0.09] bg-[#101318] pl-10 pr-4 text-sm text-zinc-200 outline-none placeholder:text-zinc-700 focus:border-cyan-300/60"
              />
            </div>

            <div className="flex gap-1 overflow-x-auto pb-1" aria-label="Preset category filter">
              {presetCategories.map((item) => (
                <button
                  key={item}
                  type="button"
                  aria-pressed={category === item}
                  onClick={() => setCategory(item)}
                  className={`h-9 shrink-0 rounded-lg px-3 text-xs transition-colors active:translate-y-px ${
                    category === item
                      ? "bg-cyan-300/12 font-medium text-cyan-200"
                      : "text-zinc-600 hover:bg-white/[0.04] hover:text-zinc-300"
                  }`}
                >
                  {item}
                </button>
              ))}
            </div>
          </div>

          {filteredPresets.length ? (
            <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {filteredPresets.map((preset) => (
                <EffectCard
                  key={preset.id}
                  preset={preset}
                  selected={selectedPreset.id === preset.id}
                  onSelect={selectPreset}
                />
              ))}
            </div>
          ) : (
            <div className="mt-5 rounded-2xl border border-dashed border-white/10 py-16 text-center">
              <p className="text-sm text-zinc-400">No presets match this search.</p>
              <button
                type="button"
                onClick={() => {
                  setQuery("");
                  setCategory("All");
                }}
                className="mt-3 text-xs font-medium text-cyan-300 hover:text-cyan-200"
              >
                Clear filters
              </button>
            </div>
          )}
        </section>

        <aside className="h-fit rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 xl:sticky xl:top-6">
          <div className="flex items-center justify-between gap-4">
            <p className="text-xs font-medium text-zinc-500">Preset details</p>
            <SlidersHorizontal size={17} className="text-zinc-700" aria-hidden="true" />
          </div>
          <h2 className="mt-6 text-lg font-semibold tracking-[-0.02em] text-zinc-100">
            {selectedPreset.name}
          </h2>
          <p className="mt-2 text-xs leading-5 text-zinc-500">{selectedPreset.description}</p>
          <dl className="mt-6 space-y-4 border-t border-white/[0.07] pt-5 text-xs">
            <div className="flex justify-between gap-4">
              <dt className="text-zinc-600">Category</dt>
              <dd className="text-zinc-300">{selectedPreset.category}</dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-zinc-600">Pitch</dt>
              <dd className="font-mono text-zinc-300">{selectedPreset.pitch} st</dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-zinc-600">Formant</dt>
              <dd className="font-mono text-zinc-300">{selectedPreset.formant}%</dd>
            </div>
          </dl>
          <button
            type="button"
            disabled
            className="mt-7 h-10 w-full rounded-xl bg-cyan-300/15 text-sm font-medium text-cyan-200/45 disabled:cursor-not-allowed"
          >
            Apply preset - Coming Soon
          </button>
          <p className="mt-3 text-center text-[11px] leading-4 text-zinc-700">
            Applying presets requires the audio engine.
          </p>
        </aside>
      </div>
    </div>
  );
}
