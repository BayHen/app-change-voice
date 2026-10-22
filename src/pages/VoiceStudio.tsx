import { FloppyDisk, Headphones, Info, ArrowCounterClockwise } from "@phosphor-icons/react";
import { AppHeader } from "../components/AppHeader";
import { StatusIndicator } from "../components/StatusIndicator";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { VoiceControls, type VoiceControlValues } from "../components/VoiceControls";
import { useAudioRuntime } from "../hooks/useAudioRuntime";
import { useEffectParameters } from "../hooks/useEffectParameters";

export function VoiceStudio() {
  const effects = useEffectParameters();
  const runtime = useAudioRuntime();
  const values: VoiceControlValues = {
    pitch: effects.parameters.pitchSemitones,
    formant: effects.parameters.formantSemitones,
    bass: 0,
    treble: 0,
    gain: 0,
  };
  const processing =
    runtime.status.state === "running" &&
    effects.parameters.enabled &&
    !effects.parameters.bypassed;

  const updateControls = (next: VoiceControlValues) => {
    void effects.update({
      ...effects.parameters,
      pitchSemitones: next.pitch,
      formantSemitones: next.formant,
    });
  };

  return (
    <div className="mx-auto max-w-[1180px]">
      <AppHeader
        title="Voice Studio"
        description="Realtime pitch and formant processing runs locally in the Rust audio pipeline."
        action={
          <StatusIndicator
            label={effects.loading ? "Loading DSP" : processing ? "Processing live" : "DSP idle"}
            tone={
              effects.error
                ? "error"
                : effects.loading
                  ? "loading"
                  : processing
                    ? "online"
                    : "offline"
            }
          />
        }
      />

      {effects.error ? (
        <p
          role="alert"
          className="mt-4 rounded-xl border border-rose-400/20 bg-rose-400/[0.06] p-3 text-xs text-rose-200"
        >
          {effects.error}
        </p>
      ) : null}

      <div className="mt-6 grid gap-5 xl:grid-cols-[1fr_300px]">
        <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-7">
          <div className="flex items-start gap-3 border-b border-white/[0.07] pb-5">
            <Info size={18} className="mt-0.5 shrink-0 text-cyan-300" aria-hidden="true" />
            <p className="text-xs leading-5 text-zinc-500">
              Pitch preserves stream duration; formant changes vocal resonance independently. Use
              headphones before starting monitoring on Dashboard.
            </p>
          </div>
          <div className="mt-7">
            <VoiceControls values={values} disabled={effects.loading} onChange={updateControls} />
          </div>
        </section>

        <aside className="h-fit rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5">
          <h2 className="text-sm font-semibold text-zinc-200">Processing</h2>
          <div className="mt-5 space-y-5">
            <ToggleSwitch
              label="Effect enabled"
              description="Runs pitch and formant processing in the active audio stream."
              checked={effects.parameters.enabled}
              disabled={effects.loading}
              onChange={(enabled) => void effects.update({ ...effects.parameters, enabled })}
            />
            <ToggleSwitch
              label="Effect bypass"
              description="Crossfades to a latency-aligned dry signal."
              checked={effects.parameters.bypassed}
              disabled={effects.loading || !effects.parameters.enabled}
              onChange={(bypassed) => void effects.update({ ...effects.parameters, bypassed })}
            />
          </div>
          <div className="mt-6 border-t border-white/[0.07] pt-5">
            <p className="text-xs text-zinc-600">Current DSP values</p>
            <div className="mt-4 grid grid-cols-2 gap-x-4 gap-y-3 font-mono text-xs">
              <span className="text-zinc-600">Pitch</span>
              <span className="text-right text-zinc-300">{values.pitch.toFixed(1)} st</span>
              <span className="text-zinc-600">Formant</span>
              <span className="text-right text-zinc-300">{values.formant.toFixed(1)} st</span>
              <span className="text-zinc-600">Preview</span>
              <span className="text-right text-zinc-300">{processing ? "Live" : "Stopped"}</span>
            </div>
          </div>
          <button
            type="button"
            onClick={() => void effects.reset()}
            disabled={effects.loading}
            className="mt-6 inline-flex h-10 w-full items-center justify-center gap-2 rounded-xl border border-white/10 text-sm font-medium text-zinc-300 hover:bg-white/[0.04] disabled:cursor-not-allowed disabled:opacity-40"
          >
            <ArrowCounterClockwise size={16} aria-hidden="true" /> Reset defaults
          </button>
          <div className="mt-4 flex items-center gap-2 rounded-xl border border-amber-300/10 bg-amber-300/[0.04] p-3 text-xs text-amber-100/60">
            <Headphones size={16} aria-hidden="true" /> Monitoring is controlled on Dashboard.
          </div>
          <button
            type="button"
            disabled
            className="mt-4 inline-flex h-10 w-full items-center justify-center gap-2 rounded-xl bg-cyan-300/15 text-sm font-medium text-cyan-200/45 disabled:cursor-not-allowed"
          >
            <FloppyDisk size={16} aria-hidden="true" /> Save preset — Coming Soon
          </button>
        </aside>
      </div>
    </div>
  );
}
