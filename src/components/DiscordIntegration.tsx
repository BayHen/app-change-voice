import { ArrowClockwise, Broadcast, DiscordLogo, Warning } from "@phosphor-icons/react";
import { useAudioRuntime } from "../hooks/useAudioRuntime";
import { useVirtualRouting } from "../hooks/useVirtualRouting";
import { AudioMeter } from "./AudioMeter";
import { DeviceSelector } from "./DeviceSelector";
import { StatusIndicator } from "./StatusIndicator";

export function DiscordIntegration() {
  const routing = useVirtualRouting();
  const runtime = useAudioRuntime();
  const active = routing.status.state === "active";
  const signalDetected = active && routing.status.outputPeakDbfs > -55;
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
  if (!options.length)
    options.push({
      id: "none",
      name: "No output endpoints found",
      kind: "output",
      available: false,
      isDefault: false,
    });
  const selected = routing.configuration.outputDeviceId ?? "none";
  const selectedDevice = routing.devices.find((device) => device.id === selected);
  const meter = Math.max(0, Math.min(100, ((routing.status.outputPeakDbfs + 60) / 60) * 100));

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <DiscordLogo size={22} className="text-indigo-300" weight="fill" />
          <div>
            <p className="text-sm font-medium text-zinc-200">Discord microphone bridge</p>
            <p className="mt-1 text-xs text-zinc-600">
              Windows virtual audio device; no Discord API or bot.
            </p>
          </div>
        </div>
        <StatusIndicator
          label={
            routing.status.state === "faulted"
              ? "Routing error"
              : active
                ? "Routing active"
                : selectedDevice?.likelyVirtual
                  ? "Device available"
                  : "Device missing"
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
      </div>

      {routing.error ? (
        <p role="alert" className="rounded-xl bg-rose-300/[.07] px-4 py-3 text-xs text-rose-200">
          {routing.error}
        </p>
      ) : null}
      <div className="grid gap-4 lg:grid-cols-[1fr_.8fr]">
        <div>
          <div className="flex items-end gap-2">
            <div className="min-w-0 flex-1">
              <DeviceSelector
                id="discord-virtual-output"
                label="VB-CABLE playback endpoint"
                value={selected}
                devices={options}
                disabled={routing.loading || routing.busy || active}
                onChange={(id) => void routing.select(id)}
                helperText={
                  selectedDevice?.likelyVirtual
                    ? "Discord must use the paired capture endpoint: CABLE Output."
                    : "Select CABLE Input. A normal speaker does not create a microphone."
                }
              />
            </div>
            <button
              aria-label="Refresh Discord audio devices"
              className="mb-1 rounded-lg border border-white/10 p-2.5 text-zinc-500"
              disabled={routing.busy}
              onClick={() => void routing.refresh()}
            >
              <ArrowClockwise />
            </button>
          </div>
          <div className="mt-4 flex gap-2">
            <button
              type="button"
              disabled={routing.busy || selected === "none" || runtime.status.state !== "running"}
              onClick={() => void (active ? routing.stop() : routing.start())}
              className="inline-flex items-center gap-2 rounded-xl bg-indigo-300 px-4 py-2.5 text-sm font-medium text-indigo-950 disabled:bg-zinc-800 disabled:text-zinc-600"
            >
              <Broadcast />
              {active ? "Stop routing" : "Start routing"}
            </button>
            <button
              type="button"
              onClick={() => void routing.openSoundSettings()}
              className="rounded-xl border border-white/10 px-4 py-2.5 text-sm text-zinc-300"
            >
              Windows Sound Settings
            </button>
          </div>
          {runtime.status.state !== "running" ? (
            <p className="mt-3 text-xs text-amber-200/70">
              Start Monitoring on Dashboard first. Routing reuses that processed pipeline.
            </p>
          ) : null}
        </div>
        <div className="rounded-xl border border-white/[.07] bg-black/15 p-4">
          <AudioMeter label="Processed signal to Discord cable" level={meter} disabled={!active} />
          <div className="mt-4 flex items-center justify-between text-xs">
            <span className="text-zinc-600">Audio signal</span>
            <span className={signalDetected ? "text-emerald-300" : "text-zinc-500"}>
              {signalDetected ? "Detected" : "Not detected"}
            </span>
          </div>
          <div className="mt-2 flex items-center justify-between text-xs">
            <span className="text-zinc-600">Peak</span>
            <span className="font-mono text-zinc-300">
              {routing.status.outputPeakDbfs.toFixed(1)} dBFS
            </span>
          </div>
        </div>
      </div>

      <div className="rounded-xl border border-indigo-300/10 bg-indigo-300/[.035] p-4 text-xs leading-5 text-zinc-400">
        <p className="font-medium text-zinc-200">Discord configuration</p>
        <ol className="mt-2 list-decimal space-y-1 pl-4">
          <li>
            Voice &amp; Video → Input Device:{" "}
            <strong className="text-zinc-200">CABLE Output (VB-Audio Virtual Cable)</strong>.
          </li>
          <li>Output Device: your physical headphones—never CABLE Input.</li>
          <li>
            Run Mic Test. Disable Discord Noise Suppression and Automatic Gain Control temporarily
            when comparing VoiceMorph effects.
          </li>
          <li>
            Adjust Input Sensitivity so quiet processed speech is detected without opening on
            background noise.
          </li>
        </ol>
        <p className="mt-3 flex gap-2 text-amber-200/70">
          <Warning className="mt-0.5 shrink-0" />
          Routing Active only proves that VoiceMorph opened the playback endpoint. It does not claim
          Discord received audio until its Mic Test or meter confirms it.
        </p>
      </div>
    </div>
  );
}
