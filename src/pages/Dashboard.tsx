import {
  ArrowRight,
  ArrowClockwise,
  MicrophoneSlash,
  Record,
  SlidersHorizontal,
  SpeakerHigh,
} from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import { AppHeader } from "../components/AppHeader";
import { AudioMeter } from "../components/AudioMeter";
import { DeviceSelector } from "../components/DeviceSelector";
import { StatusIndicator } from "../components/StatusIndicator";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { VolumeSlider } from "../components/VolumeSlider";
import { useAppStatus } from "../hooks/useAppStatus";
import { useAudioDevices } from "../hooks/useAudioDevices";
import type { AudioDevice, AudioUiState, DeviceOption } from "../types/audio";

const unavailableInput: DeviceOption[] = [
  { id: "none-input", name: "No microphone detected", kind: "input", available: false },
];

const unavailableOutput: DeviceOption[] = [
  { id: "none-output", name: "No output connected", kind: "output", available: false },
];

const initialAudioUiState: AudioUiState = {
  inputDeviceId: "",
  outputDeviceId: "",
  microphoneVolume: 0,
  outputVolume: 0,
  noiseSuppression: false,
  monitoring: false,
};

interface DashboardProps {
  onNotify?: (title: string, message: string) => void;
}

function toOptions(devices: AudioDevice[]): DeviceOption[] {
  return devices.map((device) => ({
    id: device.id,
    name: device.name,
    kind: device.kind,
    available: true,
    isDefault: device.isDefault,
  }));
}

function deviceDetails(device: AudioDevice | undefined): string {
  if (!device) return "No device selected.";
  const values = [
    device.sampleRate ? `${device.sampleRate.toLocaleString()} Hz` : null,
    device.channels ? `${device.channels} channel${device.channels === 1 ? "" : "s"}` : null,
    device.sampleFormat,
    device.bufferCapability?.minimumFrames && device.bufferCapability.maximumFrames
      ? `${device.bufferCapability.minimumFrames}-${device.bufferCapability.maximumFrames} frame buffer`
      : null,
  ];
  return values.filter(Boolean).join(" · ") || "Driver did not report format capabilities.";
}

export function Dashboard({ onNotify }: DashboardProps) {
  const { loading, error } = useAppStatus();
  const devices = useAudioDevices();
  const [audioUi, setAudioUi] = useState<AudioUiState>(initialAudioUiState);
  const inputDevices = useMemo(
    () => devices.snapshot?.inputDevices ?? [],
    [devices.snapshot?.inputDevices],
  );
  const outputDevices = useMemo(
    () => devices.snapshot?.outputDevices ?? [],
    [devices.snapshot?.outputDevices],
  );
  const selectedInputId = devices.snapshot?.inputSelection.selectedId ?? "";
  const selectedOutputId = devices.snapshot?.outputSelection.selectedId ?? "";
  const selectedInput = inputDevices.find((device) => device.id === selectedInputId);
  const selectedOutput = outputDevices.find((device) => device.id === selectedOutputId);
  const inputOptions = useMemo(
    () => (inputDevices.length ? toOptions(inputDevices) : unavailableInput),
    [inputDevices],
  );
  const outputOptions = useMemo(
    () => (outputDevices.length ? toOptions(outputDevices) : unavailableOutput),
    [outputDevices],
  );

  useEffect(() => {
    const fallbackMessage =
      devices.snapshot?.inputSelection.message ?? devices.snapshot?.outputSelection.message;
    if (fallbackMessage) onNotify?.("Audio device selection updated", fallbackMessage);
  }, [devices.snapshot, onNotify]);

  return (
    <div className="mx-auto max-w-[1180px]">
      <AppHeader
        title="Dashboard"
        description="Configure the signal path and check connection status before starting a session."
        action={
          <StatusIndicator
            label={
              loading || devices.loading
                ? "Checking backend"
                : error || devices.error
                  ? "Backend unavailable"
                  : inputDevices.length === 0
                    ? "No microphone"
                    : "Ready"
            }
            tone={
              loading || devices.loading
                ? "loading"
                : error || devices.error
                  ? "error"
                  : inputDevices.length === 0
                    ? "offline"
                    : "online"
            }
          />
        }
      />

      {loading || devices.loading ? (
        <div className="mt-6 grid gap-3" aria-label="Loading audio status" aria-busy="true">
          <div className="h-16 animate-pulse rounded-2xl bg-white/[0.035]" />
          <div className="h-44 animate-pulse rounded-2xl bg-white/[0.025]" />
        </div>
      ) : (
        <>
          {inputDevices.length === 0 || devices.error ? (
            <section
              role="alert"
              className="mt-6 flex items-start gap-4 rounded-2xl border border-amber-300/15 bg-amber-300/[0.045] p-4"
            >
              <span className="grid size-9 shrink-0 place-items-center rounded-xl bg-amber-300/10 text-amber-200">
                <MicrophoneSlash size={18} aria-hidden="true" />
              </span>
              <div>
                <h2 className="text-sm font-medium text-zinc-200">Microphone is not connected</h2>
                <p className="mt-1 text-xs leading-5 text-zinc-500">
                  Audio controls remain disabled until a backend device connection is available.
                </p>
                {error || devices.error ? (
                  <p className="mt-1 text-xs text-rose-300">
                    {devices.error ??
                      "Desktop bridge unavailable. Open the interface through Tauri to check devices."}
                  </p>
                ) : null}
              </div>
            </section>
          ) : null}

          <div className="mt-5 grid gap-5 xl:grid-cols-[1.15fr_0.85fr]">
            <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
              <div className="flex items-center justify-between gap-4">
                <div>
                  <h2 className="text-sm font-semibold text-zinc-200">Signal path</h2>
                  <p className="mt-1 text-xs text-zinc-600">Input and output device routing</p>
                </div>
                <SlidersHorizontal size={19} className="text-zinc-700" aria-hidden="true" />
              </div>

              <div className="mt-4 flex justify-end">
                <button
                  type="button"
                  onClick={() => void devices.refresh()}
                  disabled={devices.refreshing}
                  className="inline-flex h-9 items-center gap-2 rounded-lg border border-white/[0.09] px-3 text-xs text-zinc-400 hover:border-cyan-300/30 hover:text-cyan-200 disabled:cursor-wait disabled:opacity-50"
                >
                  <ArrowClockwise
                    size={14}
                    className={devices.refreshing ? "animate-spin" : ""}
                    aria-hidden="true"
                  />
                  {devices.refreshing ? "Refreshing" : "Refresh devices"}
                </button>
              </div>

              <div className="mt-4 grid gap-5 sm:grid-cols-2">
                <DeviceSelector
                  id="input-device"
                  label="Input device"
                  value={selectedInputId || "none-input"}
                  devices={inputOptions}
                  disabled={devices.refreshing || inputDevices.length === 0}
                  helperText={deviceDetails(selectedInput)}
                  onChange={(id) => void devices.select("input", id)}
                />
                <DeviceSelector
                  id="output-device"
                  label="Output device"
                  value={selectedOutputId || "none-output"}
                  devices={outputOptions}
                  disabled={devices.refreshing || outputDevices.length === 0}
                  helperText={deviceDetails(selectedOutput)}
                  onChange={(id) => void devices.select("output", id)}
                />
              </div>

              <div className="mt-7 grid gap-6 border-t border-white/[0.07] pt-6 sm:grid-cols-2">
                <VolumeSlider
                  id="microphone-volume"
                  label="Microphone volume"
                  value={audioUi.microphoneVolume}
                  disabled
                  onChange={(microphoneVolume) => setAudioUi({ ...audioUi, microphoneVolume })}
                />
                <VolumeSlider
                  id="output-volume"
                  label="Output volume"
                  value={audioUi.outputVolume}
                  disabled
                  onChange={(outputVolume) => setAudioUi({ ...audioUi, outputVolume })}
                />
              </div>

              <div className="mt-7 border-t border-white/[0.07] pt-6">
                <ToggleSwitch
                  label="Noise suppression"
                  description="Requires the audio processing backend."
                  checked={audioUi.noiseSuppression}
                  disabled
                  onChange={(noiseSuppression) => setAudioUi({ ...audioUi, noiseSuppression })}
                />
              </div>
            </section>

            <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
              <div className="flex items-center justify-between gap-4">
                <div>
                  <h2 className="text-sm font-semibold text-zinc-200">Live levels</h2>
                  <p className="mt-1 text-xs text-zinc-600">
                    Meters show zero while audio is offline
                  </p>
                </div>
                <SpeakerHigh size={19} className="text-zinc-700" aria-hidden="true" />
              </div>
              <div className="mt-7 space-y-7">
                <AudioMeter label="Input level" level={0} disabled />
                <AudioMeter label="Output level" level={0} disabled />
              </div>
            </section>
          </div>

          <section className="mt-5 grid gap-5 rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6 lg:grid-cols-[1fr_auto] lg:items-center">
            <div className="flex items-center gap-4">
              <div className="grid size-11 shrink-0 place-items-center rounded-xl border border-cyan-300/15 bg-cyan-300/[0.06] text-cyan-300">
                <span className="font-mono text-xs font-semibold">CV</span>
              </div>
              <div>
                <p className="text-xs text-zinc-600">Current preset</p>
                <h2 className="mt-1 text-sm font-semibold text-zinc-200">Clean Voice</h2>
                <p className="mt-1 text-xs text-zinc-600">
                  Neutral local preset, no processing applied.
                </p>
              </div>
            </div>

            <div className="flex flex-col gap-2 sm:flex-row">
              <button
                type="button"
                disabled
                className="inline-flex h-10 items-center justify-center gap-2 rounded-xl border border-white/[0.09] px-4 text-sm text-zinc-600 disabled:cursor-not-allowed"
              >
                Start monitoring
                <ArrowRight size={15} aria-hidden="true" />
              </button>
              <button
                type="button"
                disabled
                className="inline-flex h-10 items-center justify-center gap-2 rounded-xl bg-cyan-300/20 px-4 text-sm font-medium text-cyan-200/50 disabled:cursor-not-allowed"
              >
                <Record size={16} weight="fill" aria-hidden="true" />
                Record &amp; Test
                <span className="text-[10px] uppercase tracking-wide">Soon</span>
              </button>
            </div>
          </section>
        </>
      )}
    </div>
  );
}
