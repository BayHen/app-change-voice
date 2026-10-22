import {
  ArrowCounterClockwise,
  DownloadSimple,
  Heart,
  MagnifyingGlass,
  Plus,
  SlidersHorizontal,
  UploadSimple,
} from "@phosphor-icons/react";
import { useMemo, useState } from "react";
import { AppHeader } from "../components/AppHeader";
import { EffectCard } from "../components/EffectCard";
import { PresetEditor } from "../components/PresetEditor";
import { usePresets } from "../hooks/usePresets";
import type { PresetCategory, PresetEffect, VoicePresetSummary } from "../types/preset";

interface VoiceEffectsProps {
  onPresetSelected: (preset: VoicePresetSummary) => void;
}
const categories: Array<"All" | PresetCategory> = [
  "All",
  "Natural",
  "Character",
  "Creative",
  "Utility",
];

function effectName(effect: PresetEffect) {
  if (effect.type === "ringModulator") return `Ring mod ${effect.frequencyHz} Hz`;
  if (effect.type === "radioFilter") return `Radio ${effect.highPassHz}–${effect.lowPassHz} Hz`;
  return `Tremolo ${effect.rateHz} Hz`;
}

export function VoiceEffects({ onPresetSelected }: VoiceEffectsProps) {
  const library = usePresets();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<"All" | PresetCategory>("All");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editor, setEditor] = useState<{
    mode: "create" | "edit";
    preset: VoicePresetSummary;
  } | null>(null);

  const effectiveSelectedId = selectedId ?? library.activeId ?? library.presets[0]?.id;
  const selectedPreset =
    library.presets.find((preset) => preset.id === effectiveSelectedId) ?? library.presets[0];
  const filteredPresets = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return library.presets.filter(
      (preset) =>
        (category === "All" || preset.category === category) &&
        (!normalized ||
          preset.name.toLowerCase().includes(normalized) ||
          preset.description.toLowerCase().includes(normalized)),
    );
  }, [category, library.presets, query]);

  async function applySelected() {
    if (!selectedPreset) return;
    const applied = await library.apply(selectedPreset.id);
    if (applied) onPresetSelected(applied);
  }

  async function resetNatural() {
    const applied = await library.reset();
    if (applied) {
      setSelectedId(applied.id);
      onPresetSelected(applied);
    }
  }

  function blankPreset(): VoicePresetSummary {
    return {
      schemaVersion: 2,
      id: "custom",
      name: "My Voice",
      description: "Custom local voice preset.",
      category: "Character",
      pitchSemitones: 0,
      formantSemitones: 0,
      gain: { makeupDb: 0 },
      effects: [],
      builtIn: false,
      createdAt: "",
      updatedAt: "",
      favorite: false,
      parameters: {
        inputGainPercent: 100,
        outputGainPercent: 100,
        bassDb: 0,
        trebleDb: 0,
        noiseSuppression: false,
        suppressionStrength: 0.8,
        noiseGate: true,
        gateThresholdDbfs: -52,
        limiterEnabled: true,
        limiterCeilingDbfs: -1,
        effectBypassed: false,
      },
      pluginRequirements: [],
    };
  }

  return (
    <div className="mx-auto max-w-[1180px]">
      <AppHeader
        title="Voice Effects"
        description="Validated built-in DSP presets. Effects run locally and are not AI voice conversion."
      />
      {library.error ? (
        <p
          role="alert"
          className="mt-4 rounded-xl border border-rose-400/20 bg-rose-400/[0.06] p-3 text-xs text-rose-200"
        >
          {library.error}
        </p>
      ) : null}
      {library.notice ? (
        <p
          role="status"
          className="mt-4 rounded-xl border border-emerald-300/20 bg-emerald-300/[0.05] p-3 text-xs text-emerald-200"
        >
          {library.notice}
        </p>
      ) : null}

      <div className="mt-5 flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => setEditor({ mode: "create", preset: blankPreset() })}
          className="inline-flex h-10 items-center gap-2 rounded-xl bg-cyan-300 px-4 text-sm font-semibold text-[#071014]"
        >
          <Plus size={15} />
          Create preset
        </button>
        <button
          type="button"
          onClick={() => void library.importJson()}
          className="inline-flex h-10 items-center gap-2 rounded-xl border border-white/10 px-4 text-sm text-zinc-300"
        >
          <UploadSimple size={15} />
          Import JSON
        </button>
      </div>

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
              {categories.map((item) => (
                <button
                  key={item}
                  type="button"
                  aria-pressed={category === item}
                  onClick={() => setCategory(item)}
                  className={`h-9 shrink-0 rounded-lg px-3 text-xs transition-colors ${category === item ? "bg-cyan-300/12 font-medium text-cyan-200" : "text-zinc-600 hover:bg-white/[0.04] hover:text-zinc-300"}`}
                >
                  {item}
                </button>
              ))}
            </div>
          </div>

          {library.loading ? (
            <div className="mt-5 rounded-2xl border border-white/[0.08] p-10 text-center text-sm text-zinc-500">
              Loading validated presets…
            </div>
          ) : filteredPresets.length ? (
            <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {filteredPresets.map((preset) => (
                <EffectCard
                  key={preset.id}
                  preset={preset}
                  selected={selectedPreset?.id === preset.id}
                  active={library.activeId === preset.id}
                  onSelect={(item) => setSelectedId(item.id)}
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
                className="mt-3 text-xs font-medium text-cyan-300"
              >
                Clear filters
              </button>
              <div className="mt-3 grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onClick={() => void library.duplicate(selectedPreset.id)}
                  className="h-9 rounded-xl border border-white/10 text-xs text-zinc-300"
                >
                  Duplicate
                </button>
                <button
                  type="button"
                  onClick={() => void library.exportJson(selectedPreset.id)}
                  className="inline-flex h-9 items-center justify-center gap-2 rounded-xl border border-white/10 text-xs text-zinc-300"
                >
                  <DownloadSimple size={14} />
                  Export
                </button>
                {!selectedPreset.builtIn ? (
                  <>
                    <button
                      type="button"
                      onClick={() => setEditor({ mode: "edit", preset: selectedPreset })}
                      className="h-9 rounded-xl border border-white/10 text-xs text-zinc-300"
                    >
                      Edit / Rename
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        void library.favorite(selectedPreset.id, !selectedPreset.favorite)
                      }
                      className="inline-flex h-9 items-center justify-center gap-2 rounded-xl border border-white/10 text-xs text-zinc-300"
                    >
                      <Heart size={14} weight={selectedPreset.favorite ? "fill" : "regular"} />
                      {selectedPreset.favorite ? "Favorited" : "Favorite"}
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        if (window.confirm(`Delete ${selectedPreset.name}?`))
                          void library.remove(selectedPreset.id);
                      }}
                      className="col-span-2 h-9 rounded-xl border border-rose-300/15 text-xs text-rose-300"
                    >
                      Delete custom preset
                    </button>
                  </>
                ) : null}
              </div>
            </div>
          )}
        </section>

        <aside className="h-fit rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 xl:sticky xl:top-6">
          <div className="flex items-center justify-between gap-4">
            <p className="text-xs font-medium text-zinc-500">Preset details</p>
            <SlidersHorizontal size={17} className="text-zinc-700" aria-hidden="true" />
          </div>
          {selectedPreset ? (
            <>
              <h2 className="mt-6 text-lg font-semibold text-zinc-100">{selectedPreset.name}</h2>
              <p className="mt-2 text-xs leading-5 text-zinc-500">{selectedPreset.description}</p>
              <dl className="mt-6 space-y-3 border-t border-white/[0.07] pt-5 text-xs">
                <div className="flex justify-between">
                  <dt className="text-zinc-600">Pitch</dt>
                  <dd className="font-mono text-zinc-300">{selectedPreset.pitchSemitones} st</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-zinc-600">Formant</dt>
                  <dd className="font-mono text-zinc-300">{selectedPreset.formantSemitones} st</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-zinc-600">Makeup gain</dt>
                  <dd className="font-mono text-zinc-300">{selectedPreset.gain.makeupDb} dB</dd>
                </div>
                <div>
                  <dt className="text-zinc-600">Additional DSP</dt>
                  <dd className="mt-1 text-zinc-300">
                    {selectedPreset.effects.length
                      ? selectedPreset.effects.map(effectName).join(", ")
                      : "None"}
                  </dd>
                </div>
              </dl>
              <button
                type="button"
                onClick={() => void applySelected()}
                disabled={Boolean(library.applyingId) || library.activeId === selectedPreset.id}
                className="mt-7 h-10 w-full rounded-xl bg-cyan-300 text-sm font-semibold text-[#071014] disabled:cursor-not-allowed disabled:opacity-40"
              >
                {library.applyingId === selectedPreset.id
                  ? "Applying…"
                  : library.activeId === selectedPreset.id
                    ? "Active preset"
                    : "Apply preset"}
              </button>
              <button
                type="button"
                onClick={() => void resetNatural()}
                disabled={Boolean(library.applyingId) || library.activeId === "natural"}
                className="mt-3 inline-flex h-9 w-full items-center justify-center gap-2 rounded-xl border border-white/10 text-xs text-zinc-400 disabled:opacity-40"
              >
                <ArrowCounterClockwise size={14} />
                Reset to Natural
              </button>
            </>
          ) : null}
        </aside>
      </div>
      {editor ? (
        <PresetEditor
          preset={editor.preset}
          mode={editor.mode}
          onClose={() => setEditor(null)}
          onSave={editor.mode === "create" ? library.create : library.update}
          onDuplicate={library.duplicate}
        />
      ) : null}
    </div>
  );
}
