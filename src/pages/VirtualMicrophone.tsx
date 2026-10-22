import {
  ArrowClockwise,
  ArrowSquareOut,
  Broadcast,
  Headphones,
  Warning,
} from "@phosphor-icons/react";
import { AppHeader } from "../components/AppHeader";
import { AudioMeter } from "../components/AudioMeter";
import { DeviceSelector } from "../components/DeviceSelector";
import { StatusIndicator } from "../components/StatusIndicator";
import { useAudioRuntime } from "../hooks/useAudioRuntime";
import { useVirtualRouting } from "../hooks/useVirtualRouting";

export function VirtualMicrophone() {
  const routing = useVirtualRouting();
  const runtime = useAudioRuntime();
  const hasLikelyVirtualDevice = routing.devices.some((device) => device.likelyVirtual);
  const candidates = [...routing.devices].sort(
    (left, right) => Number(right.likelyVirtual) - Number(left.likelyVirtual),
  );
  const options = candidates.map((device) => ({
    id: device.id,
    name: device.name,
    kind: "output" as const,
    available: true,
    isDefault: device.isDefault,
  }));
  if (options.length === 0)
    options.push({
      id: "none",
      name: "No virtual output detected",
      kind: "output",
      available: false,
      isDefault: false,
    });
  const selected = routing.configuration.outputDeviceId ?? "none";
  const selectedDevice = routing.devices.find((device) => device.id === selected);
  const meterPercent = Math.max(
    0,
    Math.min(100, ((routing.status.outputPeakDbfs + 60) / 60) * 100),
  );
  const active = routing.status.state === "active";

  return (
    <div className="mx-auto max-w-[1060px]">
      <AppHeader
        title="Virtual Microphone"
        description="Route the processed voice to a user-installed Windows virtual audio cable."
        action={
          <StatusIndicator
            label={
              routing.status.state === "faulted"
                ? "Endpoint lost"
                : active
                  ? "Routing active"
                  : "Routing stopped"
            }
            tone={
              routing.status.state === "faulted"
                ? "error"
                : active
                  ? "online"
                  : routing.loading
                    ? "loading"
                    : "offline"
            }
          />
        }
      />

      <section className="mt-6 rounded-2xl border border-amber-300/15 bg-amber-300/[0.045] p-4 text-xs leading-5 text-amber-100/75">
        <div className="flex gap-3">
          <Warning size={19} className="mt-0.5 shrink-0 text-amber-300" aria-hidden="true" />
          <div>
            <p className="font-medium text-amber-100">
              A virtual audio driver must be installed separately
            </p>
            <p className="mt-1">
              Choose its playback endpoint here (for VB-CABLE: <strong>CABLE Input</strong>), then
              choose the paired capture endpoint (<strong>CABLE Output</strong>) as microphone in
              Discord, Zoom, Teams or OBS. VoiceMorph never installs a kernel driver automatically.
            </p>
          </div>
        </div>
      </section>

      {routing.error ? (
        <p
          role="alert"
          className="mt-4 rounded-xl border border-rose-300/20 bg-rose-300/[0.05] px-4 py-3 text-xs text-rose-200"
        >
          {routing.error}
        </p>
      ) : null}
      {!routing.loading && !hasLikelyVirtualDevice ? (
        <p
          role="alert"
          className="mt-4 rounded-xl border border-amber-300/15 bg-amber-300/[0.04] px-4 py-3 text-xs text-amber-100/70"
        >
          No recognized virtual cable is installed. Physical outputs remain visible for
          troubleshooting, but they do not create a microphone endpoint.
        </p>
      ) : null}

      <div className="mt-5 grid gap-5 lg:grid-cols-[1.1fr_0.9fr]">
        <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h2 className="text-sm font-semibold text-zinc-200">Virtual output sink</h2>
              <p className="mt-1 text-xs text-zinc-600">
                Processed signal only; input capture and DSP are shared.
              </p>
            </div>
            <button
              type="button"
              onClick={() => void routing.refresh()}
              disabled={routing.loading || routing.busy}
              aria-label="Refresh virtual devices"
              className="rounded-lg border border-white/[0.08] p-2 text-zinc-500 hover:text-cyan-200 disabled:opacity-40"
            >
              <ArrowClockwise size={16} />
            </button>
          </div>
          <div className="mt-5">
            <DeviceSelector
              id="virtual-output"
              label="Virtual cable playback endpoint"
              value={selected}
              devices={options}
              disabled={routing.loading || routing.busy || active}
              onChange={(id) => void routing.select(id)}
              helperText={
                selectedDevice && !selectedDevice.likelyVirtual
                  ? "This endpoint was not recognized as a virtual cable. Confirm that it exposes a paired capture endpoint."
                  : "Changing this endpoint stops the current audio session so streams can be rebuilt safely."
              }
            />
          </div>
          <div className="mt-6 flex flex-wrap gap-3">
            <button
              type="button"
              disabled={routing.busy || selected === "none" || runtime.status.state !== "running"}
              onClick={() => void (active ? routing.stop() : routing.start())}
              className="inline-flex h-10 items-center gap-2 rounded-xl bg-cyan-300 px-4 text-sm font-medium text-[#071014] disabled:cursor-not-allowed disabled:bg-zinc-800 disabled:text-zinc-600"
            >
              <Broadcast size={16} />
              {active ? "Stop routing" : "Start routing"}
            </button>
            <button
              type="button"
              onClick={() => void routing.openSoundSettings()}
              className="inline-flex h-10 items-center gap-2 rounded-xl border border-white/[0.09] px-4 text-sm text-zinc-300 hover:text-cyan-200"
            >
              <ArrowSquareOut size={16} />
              Windows Sound Settings
            </button>
          </div>
          {runtime.status.state !== "running" ? (
            <p className="mt-3 text-xs text-amber-200/70">
              Start monitoring/audio processing on Dashboard first. Hear Myself may remain off; the
              same DSP pipeline feeds this sink.
            </p>
          ) : null}
        </section>

        <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
          <h2 className="text-sm font-semibold text-zinc-200">Routing telemetry</h2>
          <div className="mt-6">
            <AudioMeter
              label="Processed output to virtual cable"
              level={meterPercent}
              disabled={!active}
            />
          </div>
          <dl className="mt-6 grid grid-cols-2 gap-4 border-t border-white/[0.07] pt-5 text-xs">
            <div>
              <dt className="text-zinc-600">Sink format</dt>
              <dd className="mt-1 text-zinc-300">
                {routing.status.sampleRate
                  ? `${routing.status.sampleRate.toLocaleString()} Hz / ${routing.status.channels} ch`
                  : "Not open"}
              </dd>
            </div>
            <div>
              <dt className="text-zinc-600">Peak</dt>
              <dd className="mt-1 font-mono text-zinc-300">
                {routing.status.outputPeakDbfs.toFixed(1)} dBFS
              </dd>
            </div>
            <div>
              <dt className="text-zinc-600">Underruns</dt>
              <dd className="mt-1 font-mono text-zinc-300">{routing.status.underrunCount}</dd>
            </div>
            <div>
              <dt className="text-zinc-600">Overflows</dt>
              <dd className="mt-1 font-mono text-zinc-300">{routing.status.overflowCount}</dd>
            </div>
          </dl>
          <p className="mt-5 text-[11px] leading-5 text-zinc-600">
            {routing.status.message ??
              "The app reports Active only after the playback endpoint opens. Verify the paired capture endpoint separately in the target application."}
          </p>
        </section>
      </div>

      <section className="mt-5 rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
        <div className="flex gap-3">
          <Headphones size={19} className="shrink-0 text-cyan-300" />
          <div>
            <h2 className="text-sm font-semibold text-zinc-200">Feedback prevention</h2>
            <p className="mt-2 text-xs leading-5 text-zinc-500">
              Use headphones. Do not select the virtual cable capture endpoint as VoiceMorph's
              microphone while routing to its paired playback endpoint; that creates a feedback
              loop. Device disconnect changes routing to Faulted and the sink outputs no further
              audio.
            </p>
          </div>
        </div>
      </section>
    </div>
  );
}
