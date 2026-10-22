import { FloppyDisk, Info } from "@phosphor-icons/react";
import { useState } from "react";
import { AppHeader } from "../components/AppHeader";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { VoiceControls, type VoiceControlValues } from "../components/VoiceControls";

const initialControls: VoiceControlValues = {
  pitch: 0,
  formant: 0,
  bass: 0,
  treble: 0,
  gain: 0,
};

export function VoiceStudio() {
  const [controls, setControls] = useState<VoiceControlValues>(initialControls);
  const [bypassed, setBypassed] = useState(false);

  return (
    <div className="mx-auto max-w-[1180px]">
      <AppHeader
        title="Voice Studio"
        description="Shape a local control draft. Values are not sent to audio processing in Task 02."
        action={
          <span className="inline-flex h-8 items-center rounded-full border border-white/[0.08] px-3 text-xs text-zinc-500">
            UI draft only
          </span>
        }
      />

      <div className="mt-6 grid gap-5 xl:grid-cols-[1fr_300px]">
        <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-7">
          <div className="flex items-start gap-3 border-b border-white/[0.07] pb-5">
            <Info size={18} className="mt-0.5 shrink-0 text-cyan-300" aria-hidden="true" />
            <p className="text-xs leading-5 text-zinc-500">
              These controls update local interface state for validation. They do not alter
              microphone audio.
            </p>
          </div>
          <div className="mt-7">
            <VoiceControls values={controls} onChange={setControls} />
          </div>
        </section>

        <aside className="h-fit rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5">
          <h2 className="text-sm font-semibold text-zinc-200">Processing</h2>
          <div className="mt-5">
            <ToggleSwitch
              label="Effect bypass"
              description="Local UI state only. No signal is being processed."
              checked={bypassed}
              onChange={setBypassed}
            />
          </div>
          <div className="mt-6 border-t border-white/[0.07] pt-5">
            <p className="text-xs text-zinc-600">Draft summary</p>
            <div className="mt-4 grid grid-cols-2 gap-x-4 gap-y-3 font-mono text-xs">
              <span className="text-zinc-600">Pitch</span>
              <span className="text-right text-zinc-300">{controls.pitch} st</span>
              <span className="text-zinc-600">Formant</span>
              <span className="text-right text-zinc-300">{controls.formant}%</span>
              <span className="text-zinc-600">Bypass</span>
              <span className="text-right text-zinc-300">{bypassed ? "On" : "Off"}</span>
            </div>
          </div>
          <button
            type="button"
            disabled
            className="mt-7 inline-flex h-10 w-full items-center justify-center gap-2 rounded-xl bg-cyan-300/15 text-sm font-medium text-cyan-200/45 disabled:cursor-not-allowed"
          >
            <FloppyDisk size={16} aria-hidden="true" />
            Save preset - Coming Soon
          </button>
        </aside>
      </div>
    </div>
  );
}
