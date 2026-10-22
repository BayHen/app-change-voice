import { useState } from "react";
import { previewPreset } from "../services/tauri";
import type { PresetCategory, VoicePresetSummary } from "../types/preset";

interface Props {
  preset: VoicePresetSummary;
  mode: "create" | "edit";
  onSave: (preset: VoicePresetSummary) => Promise<boolean>;
  onDuplicate: (id: string) => Promise<boolean>;
  onClose: () => void;
}
const categories: PresetCategory[] = ["Natural", "Character", "Creative", "Utility"];
export function PresetEditor({ preset, mode, onSave, onDuplicate, onClose }: Props) {
  const [draft, setDraft] = useState(preset);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const parameter = <K extends keyof VoicePresetSummary["parameters"]>(
    key: K,
    value: VoicePresetSummary["parameters"][K],
  ) => setDraft((current) => ({ ...current, parameters: { ...current.parameters, [key]: value } }));
  async function save() {
    setSaving(true);
    setError(null);
    const ok = await onSave(draft);
    setSaving(false);
    if (ok) onClose();
  }
  async function preview() {
    try {
      await previewPreset(draft);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="preset-editor-title"
      className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4"
    >
      <div className="max-h-[92vh] w-full max-w-3xl overflow-y-auto rounded-2xl border border-white/10 bg-[#101318] p-6 shadow-2xl">
        <div className="flex items-start justify-between">
          <div>
            <h2 id="preset-editor-title" className="text-lg font-semibold text-zinc-100">
              {mode === "create" ? "Create custom preset" : "Edit custom preset"}
            </h2>
            <p className="mt-1 text-xs text-zinc-500">Schema v2 · local JSON · DSP parameters</p>
          </div>
          <button type="button" onClick={onClose} className="text-sm text-zinc-500">
            Cancel
          </button>
        </div>
        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          <label className="text-xs text-zinc-400">
            Name
            <input
              aria-label="Preset name"
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
              className="mt-2 h-10 w-full rounded-lg border border-white/10 bg-black/20 px-3 text-zinc-200"
            />
          </label>
          <label className="text-xs text-zinc-400">
            Category
            <select
              aria-label="Preset category"
              value={draft.category}
              onChange={(e) => setDraft({ ...draft, category: e.target.value as PresetCategory })}
              className="mt-2 h-10 w-full rounded-lg border border-white/10 bg-black/20 px-3 text-zinc-200"
            >
              {categories.map((c) => (
                <option key={c}>{c}</option>
              ))}
            </select>
          </label>
          <label className="text-xs text-zinc-400 sm:col-span-2">
            Description
            <textarea
              aria-label="Preset description"
              value={draft.description}
              onChange={(e) => setDraft({ ...draft, description: e.target.value })}
              className="mt-2 min-h-20 w-full rounded-lg border border-white/10 bg-black/20 p-3 text-zinc-200"
            />
          </label>
        </div>
        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          {(
            [
              [
                "Pitch",
                -12,
                12,
                0.1,
                draft.pitchSemitones,
                (v: number) => setDraft({ ...draft, pitchSemitones: v }),
              ],
              [
                "Formant",
                -6,
                6,
                0.1,
                draft.formantSemitones,
                (v: number) => setDraft({ ...draft, formantSemitones: v }),
              ],
              [
                "Input gain",
                0,
                100,
                1,
                draft.parameters.inputGainPercent,
                (v: number) => parameter("inputGainPercent", v),
              ],
              [
                "Output gain",
                0,
                100,
                1,
                draft.parameters.outputGainPercent,
                (v: number) => parameter("outputGainPercent", v),
              ],
              [
                "Bass EQ",
                -12,
                12,
                0.5,
                draft.parameters.bassDb,
                (v: number) => parameter("bassDb", v),
              ],
              [
                "Treble EQ",
                -12,
                12,
                0.5,
                draft.parameters.trebleDb,
                (v: number) => parameter("trebleDb", v),
              ],
              [
                "Noise gate",
                -80,
                -20,
                1,
                draft.parameters.gateThresholdDbfs,
                (v: number) => parameter("gateThresholdDbfs", v),
              ],
              [
                "Limiter ceiling",
                -12,
                -0.1,
                0.1,
                draft.parameters.limiterCeilingDbfs,
                (v: number) => parameter("limiterCeilingDbfs", v),
              ],
            ] as const
          ).map(([label, min, max, step, value, set]) => (
            <label key={label} className="text-xs text-zinc-400">
              {label}
              <span className="float-right font-mono text-cyan-200">{value}</span>
              <input
                aria-label={label}
                type="range"
                min={min}
                max={max}
                step={step}
                value={value}
                onChange={(e) => set(Number(e.target.value))}
                className="mt-2 w-full accent-cyan-300"
              />
            </label>
          ))}
        </div>
        <div className="mt-5 grid gap-3 sm:grid-cols-2">
          {(
            [
              ["Noise suppression", "noiseSuppression"],
              ["Noise gate enabled", "noiseGate"],
              ["Limiter enabled", "limiterEnabled"],
              ["Effect bypass", "effectBypassed"],
            ] as const
          ).map(([label, key]) => (
            <label key={key} className="flex items-center gap-2 text-xs text-zinc-300">
              <input
                type="checkbox"
                checked={draft.parameters[key]}
                onChange={(e) => parameter(key, e.target.checked)}
                className="accent-cyan-300"
              />
              {label}
            </label>
          ))}
        </div>
        {error ? (
          <p role="alert" className="mt-4 text-xs text-rose-300">
            {error}
          </p>
        ) : null}
        <div className="mt-7 flex flex-wrap justify-end gap-2">
          <button
            type="button"
            onClick={() => setDraft({ ...preset, parameters: { ...preset.parameters } })}
            className="h-10 rounded-xl border border-white/10 px-4 text-sm text-zinc-400"
          >
            Reset
          </button>
          {mode === "edit" ? (
            <button
              type="button"
              onClick={() => void onDuplicate(preset.id)}
              className="h-10 rounded-xl border border-white/10 px-4 text-sm text-zinc-300"
            >
              Duplicate
            </button>
          ) : null}
          <button
            type="button"
            onClick={() => void preview()}
            className="h-10 rounded-xl border border-cyan-300/20 px-4 text-sm text-cyan-200"
          >
            Preview
          </button>
          <button
            type="button"
            disabled={saving}
            onClick={() => void save()}
            className="h-10 rounded-xl bg-cyan-300 px-5 text-sm font-semibold text-[#071014] disabled:opacity-50"
          >
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
