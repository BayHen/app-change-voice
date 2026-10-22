import {
  ArrowRight,
  ArrowClockwise,
  Headphones,
  MicrophoneSlash,
  Pause,
  Record,
  SlidersHorizontal,
  SpeakerHigh,
} from "@phosphor-icons/react";
import { useEffect, useMemo } from "react";
import { AppHeader } from "../components/AppHeader";
import { ClippingIndicator } from "../components/ClippingIndicator";
import { DeviceSelector } from "../components/DeviceSelector";
import { StatusIndicator } from "../components/StatusIndicator";
import { InputLevelMeter } from "../components/InputLevelMeter";
import { OutputLevelMeter } from "../components/OutputLevelMeter";
import { MuteButton } from "../components/MuteButton";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { VolumeSlider } from "../components/VolumeSlider";
import { useAppStatus } from "../hooks/useAppStatus";
import { useAudioDevices } from "../hooks/useAudioDevices";
import { useAudioRuntime } from "../hooks/useAudioRuntime";
import { useVolumeSettings } from "../hooks/useVolumeSettings";
import { useNoiseSettings } from "../hooks/useNoiseSettings";
import type { AudioDevice, DeviceOption } from "../types/audio";

const unavailableInput: DeviceOption[] = [
  { id: "none-input", name: "No microphone detected", kind: "input", available: false },
];

const unavailableOutput: DeviceOption[] = [
  { id: "none-output", name: "No output connected", kind: "output", available: false },
];

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
  const runtime = useAudioRuntime();
  const volume = useVolumeSettings();
  const noise = useNoiseSettings();
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
                  : runtime.status.state === "running"
                    ? "Monitoring live"
                    : runtime.status.state === "paused"
                      ? "Monitoring paused"
                      : inputDevices.length === 0
                        ? "No microphone"
                        : "Ready"
            }
            tone={
              loading || devices.loading
                ? "loading"
                : error || devices.error
                  ? "error"
                  : runtime.error
                    ? "error"
                    : runtime.status.state === "running"
                      ? "online"
                      : inputDevices.length === 0
                        ? "offline"
                        : "offline"
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

          {runtime.error ? (
            <section
              role="alert"
              className="mt-4 rounded-xl border border-rose-300/20 bg-rose-300/[0.05] px-4 py-3 text-xs text-rose-200"
            >
              Audio pipeline error: {runtime.error}
            </section>
          ) : null}
          {volume.error ? (
            <section
              role="alert"
              className="mt-4 rounded-xl border border-rose-300/20 bg-rose-300/[0.05] px-4 py-3 text-xs text-rose-200"
            >
              Volume settings error: {volume.error}
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
                <div className="space-y-3">
                  <VolumeSlider
                    id="microphone-volume"
                    label="Microphone input gain"
                    value={volume.settings.inputPercent}
                    disabled={volume.loading}
                    onChange={(inputPercent) =>
                      volume.preview({ ...volume.settings, inputPercent })
                    }
                    onCommit={(inputPercent) =>
                      void volume.commit({ ...volume.settings, inputPercent })
                    }
                  />
                  <MuteButton
                    label="microphone"
                    muted={volume.settings.inputMuted}
                    disabled={volume.loading}
                    onChange={(inputMuted) =>
                      void volume.commit({ ...volume.settings, inputMuted })
                    }
                  />
                </div>
                <div className="space-y-3">
                  <VolumeSlider
                    id="output-volume"
                    label="Output monitoring volume"
                    value={volume.settings.outputPercent}
                    disabled={volume.loading}
                    onChange={(outputPercent) =>
                      volume.preview({ ...volume.settings, outputPercent })
                    }
                    onCommit={(outputPercent) =>
                      void volume.commit({ ...volume.settings, outputPercent })
                    }
                  />
                  <MuteButton
                    label="output"
                    muted={volume.settings.outputMuted}
                    disabled={volume.loading}
                    onChange={(outputMuted) =>
                      void volume.commit({ ...volume.settings, outputMuted })
                    }
                  />
                </div>
              </div>

              <div className="mt-7 space-y-5 border-t border-white/[0.07] pt-6">
                <ToggleSwitch
                  label="Noise suppression"
                  description="RNNoise speech denoising (48 kHz internally)."
                  checked={noise.settings.enabled}
                  disabled={noise.loading}
                  onChange={(enabled) => void noise.commit({ ...noise.settings, enabled })}
                />
                <VolumeSlider
                  id="suppression-strength"
                  label="Suppression mix"
                  value={Math.round(noise.settings.suppressionStrength * 100)}
                  disabled={noise.loading || !noise.settings.enabled}
                  onChange={(value) =>
                    noise.preview({ ...noise.settings, suppressionStrength: value / 100 })
                  }
                  onCommit={(value) =>
                    void noise.commit({ ...noise.settings, suppressionStrength: value / 100 })
                  }
                />
                <ToggleSwitch
                  label="Noise processor bypass"
                  description="Latency-aligned dry comparison."
                  checked={noise.settings.bypassed}
                  disabled={noise.loading || !noise.settings.enabled}
                  onChange={(bypassed) => void noise.commit({ ...noise.settings, bypassed })}
                />
                <ToggleSwitch
                  label="Noise gate"
                  description={`Closes below ${noise.settings.gateThresholdDbfs.toFixed(0)} dBFS.`}
                  checked={noise.settings.gateEnabled}
                  disabled={noise.loading}
                  onChange={(gateEnabled) => void noise.commit({ ...noise.settings, gateEnabled })}
                />
                <label className="block text-xs text-zinc-400" htmlFor="gate-threshold">
                  Gate threshold{" "}
                  <span className="float-right font-mono text-cyan-200">
                    {noise.settings.gateThresholdDbfs.toFixed(0)} dBFS
                  </span>
                  <input
                    id="gate-threshold"
                    className="mt-3 w-full accent-cyan-300"
                    type="range"
                    min="-80"
                    max="-20"
                    step="1"
                    value={noise.settings.gateThresholdDbfs}
                    disabled={noise.loading || !noise.settings.gateEnabled}
                    onChange={(event) =>
                      noise.preview({
                        ...noise.settings,
                        gateThresholdDbfs: Number(event.target.value),
                      })
                    }
                    onPointerUp={(event) =>
                      void noise.commit({
                        ...noise.settings,
                        gateThresholdDbfs: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <ToggleSwitch
                  label="Output limiter"
                  description={`Final clipping protection at ${noise.settings.limiterCeilingDbfs.toFixed(1)} dBFS.`}
                  checked={noise.settings.limiterEnabled}
                  disabled={noise.loading}
                  onChange={(limiterEnabled) =>
                    void noise.commit({ ...noise.settings, limiterEnabled })
                  }
                />
                {noise.error ? (
                  <p role="alert" className="text-xs text-rose-300">
                    Noise settings error: {noise.error}
                  </p>
                ) : null}
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
                <div className="space-y-2">
                  <InputLevelMeter
                    peakDbfs={runtime.status.metrics?.inputPeakDbfs ?? -96}
                    rmsDbfs={runtime.status.metrics?.inputRmsDbfs ?? -96}
                    disabled={runtime.status.state !== "running"}
                  />
                  <ClippingIndicator clipping={runtime.status.metrics?.inputClipping ?? false} />
                </div>
                <div className="space-y-2">
                  <OutputLevelMeter
                    peakDbfs={runtime.status.metrics?.outputPeakDbfs ?? -96}
                    rmsDbfs={runtime.status.metrics?.outputRmsDbfs ?? -96}
                    disabled={runtime.status.state !== "running"}
                  />
                  <ClippingIndicator clipping={runtime.status.metrics?.outputClipping ?? false} />
                </div>
              </div>
              {runtime.status.metrics ? (
                <dl className="mt-7 grid grid-cols-2 gap-x-4 gap-y-3 border-t border-white/[0.07] pt-5 text-xs">
                  <div>
                    <dt className="text-zinc-600">Sample rate</dt>
                    <dd className="mt-1 text-zinc-300">
                      {runtime.status.metrics.sampleRate.toLocaleString()} Hz
                    </dd>
                  </div>
                  <div>
                    <dt className="text-zinc-600">Estimated latency</dt>
                    <dd className="mt-1 text-zinc-300">
                      {runtime.status.metrics.estimatedLatencyMs.toFixed(1)} ms
                    </dd>
                  </div>
                  <div>
                    <dt className="text-zinc-600">Underruns</dt>
                    <dd className="mt-1 font-mono text-zinc-300">
                      {runtime.status.metrics.underrunCount}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-zinc-600">Overflows</dt>
                    <dd className="mt-1 font-mono text-zinc-300">
                      {runtime.status.metrics.overflowCount}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-zinc-600">Processing/frame</dt>
                    <dd className="mt-1 text-zinc-300">
                      {runtime.status.metrics.averageProcessingMicros.toFixed(2)} µs
                    </dd>
                  </div>
                  <div>
                    <dt className="text-zinc-600">Ring depth</dt>
                    <dd className="mt-1 text-zinc-300">
                      {runtime.status.metrics.ringDepthFrames} frames
                    </dd>
                  </div>
                  <div>
                    <dt className="text-zinc-600">Limiter protection</dt>
                    <dd className="mt-1 font-mono text-zinc-300">
                      {runtime.status.metrics.protectedClipCount} peaks
                    </dd>
                  </div>
                </dl>
              ) : null}
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
              <div className="mr-2 flex max-w-[250px] items-center gap-2 text-[11px] leading-4 text-amber-200/70">
                <Headphones size={17} className="shrink-0" aria-hidden="true" />
                Use headphones before monitoring to prevent acoustic feedback.
              </div>
              {runtime.status.state !== "stopped" ? (
                <button
                  type="button"
                  disabled={runtime.busy}
                  onClick={() =>
                    void (runtime.status.state === "running" ? runtime.pause() : runtime.resume())
                  }
                  className="inline-flex h-10 items-center justify-center gap-2 rounded-xl border border-white/[0.09] px-4 text-sm text-zinc-300 disabled:cursor-wait disabled:opacity-50"
                >
                  <Pause size={15} aria-hidden="true" />
                  {runtime.status.state === "running" ? "Pause" : "Resume"}
                </button>
              ) : null}
              <button
                type="button"
                disabled={runtime.busy || !selectedInputId || !selectedOutputId}
                onClick={() =>
                  void (runtime.status.state === "stopped"
                    ? runtime.start(selectedInputId, selectedOutputId)
                    : runtime.stop())
                }
                className="inline-flex h-10 items-center justify-center gap-2 rounded-xl border border-white/[0.09] px-4 text-sm text-zinc-300 hover:border-cyan-300/30 hover:text-cyan-200 disabled:cursor-not-allowed disabled:text-zinc-600"
              >
                {runtime.status.state === "stopped" ? "Start monitoring" : "Stop monitoring"}
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
