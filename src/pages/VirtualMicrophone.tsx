import {
  ArrowClockwise,
  ArrowSquareOut,
  Broadcast,
  Link,
  Plugs,
  ShieldCheck,
  Warning,
} from "@phosphor-icons/react";
import type { ReactNode } from "react";
import { AppHeader } from "../components/AppHeader";
import { AudioMeter } from "../components/AudioMeter";
import { StatusIndicator } from "../components/StatusIndicator";
import { useAudioRuntime } from "../hooks/useAudioRuntime";
import { useVirtualMicrophone } from "../hooks/useVirtualMicrophone";

const stateLabels = {
  notInstalled: "Not installed",
  installed: "Installed",
  driverUnavailable: "Driver unavailable",
  ready: "Ready",
  bridgeConnected: "Bridge connected",
  streaming: "Streaming",
  error: "Error",
};

export function VirtualMicrophone() {
  const virtualMic = useVirtualMicrophone();
  const runtime = useAudioRuntime();
  const status = virtualMic.status;
  const metrics = virtualMic.metrics;
  const running = runtime.status.state === "running";
  const inputLevel = runtime.status.metrics?.inputLevel ?? 0;
  const outputLevel = runtime.status.metrics?.outputLevel ?? 0;
  const estimatedLatency =
    (runtime.status.metrics?.estimatedLatencyMs ?? 0) + metrics.bridgeBufferEstimateMs;
  const canConnect = ["installed", "ready"].includes(status.state) && status.versionCompatible;

  return (
    <div className="mx-auto max-w-[1120px]">
      <AppHeader
        title="Virtual Microphone"
        description="Manage the native VoiceMorph driver and route processed audio to Windows applications."
        action={
          <StatusIndicator
            label={stateLabels[status.state]}
            tone={
              status.state === "streaming"
                ? "online"
                : virtualMic.loading || virtualMic.busy
                  ? "loading"
                  : ["error", "driverUnavailable"].includes(status.state)
                    ? "error"
                    : status.state === "ready" || status.state === "bridgeConnected"
                      ? "online"
                      : "offline"
            }
          />
        }
      />

      {virtualMic.error ? (
        <p
          role="alert"
          className="mt-4 rounded-xl border border-rose-300/20 bg-rose-300/[0.05] px-4 py-3 text-xs text-rose-200"
        >
          {virtualMic.error}
        </p>
      ) : null}

      {status.state === "notInstalled" ? (
        <section className="mt-5 rounded-2xl border border-amber-300/15 bg-amber-300/[0.045] p-5 text-sm text-amber-100/75">
          <div className="flex gap-3">
            <Warning size={20} className="shrink-0 text-amber-300" />
            <div>
              <h2 className="font-semibold text-amber-100">Signed driver package required</h2>
              <p className="mt-2 text-xs leading-5">
                VoiceMorph does not install an unsigned prototype or change Windows driver-signing
                settings automatically. Install a valid signed package using the documented release
                process, then select Check Driver.
              </p>
            </div>
          </div>
        </section>
      ) : null}

      <div className="mt-5 grid gap-5 lg:grid-cols-[1.08fr_0.92fr]">
        <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h2 className="text-sm font-semibold text-zinc-200">Driver and bridge</h2>
              <p className="mt-1 text-xs text-zinc-600">
                Control-plane checks never start microphone capture.
              </p>
            </div>
            <ShieldCheck
              size={22}
              className={status.versionCompatible ? "text-emerald-300" : "text-zinc-700"}
            />
          </div>

          <dl className="mt-6 grid gap-4 text-xs sm:grid-cols-2">
            <Info label="Driver" value={status.driverName} />
            <Info label="Installed version" value={status.driverVersion ?? "Not detected"} />
            <Info label="Expected version" value={status.expectedDriverVersion} />
            <Info label="Protocol" value={`v${status.protocolVersion}`} />
            <Info label="Capture endpoint" value={status.captureEndpointName ?? "Not detected"} />
            <Info label="Bridge" value={status.bridgeConnected ? "Connected" : "Disconnected"} />
          </dl>

          <p className="mt-5 rounded-xl border border-white/[0.06] bg-black/20 px-4 py-3 text-xs leading-5 text-zinc-400">
            {status.message}
          </p>

          <div className="mt-5 flex flex-wrap gap-2">
            <Action
              icon={<ArrowClockwise size={15} />}
              label="Check Driver"
              onClick={virtualMic.check}
              disabled={virtualMic.busy}
            />
            <Action
              icon={<Link size={15} />}
              label="Connect"
              onClick={virtualMic.connect}
              disabled={virtualMic.busy || !canConnect}
              primary
            />
            <Action
              icon={<Plugs size={15} />}
              label="Disconnect"
              onClick={virtualMic.disconnect}
              disabled={virtualMic.busy || !status.bridgeConnected}
            />
          </div>
        </section>

        <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
          <h2 className="text-sm font-semibold text-zinc-200">Realtime signal</h2>
          <div className="mt-5 space-y-5">
            <AudioMeter
              label="Processed input"
              level={inputLevel * 100}
              disabled={!metrics.streaming}
            />
            <AudioMeter
              label="Virtual microphone output"
              level={outputLevel * 100}
              disabled={!metrics.streaming}
            />
          </div>
          <dl className="mt-6 grid grid-cols-2 gap-4 border-t border-white/[0.07] pt-5 text-xs">
            <Info
              label="Format"
              value={`${metrics.sampleRate.toLocaleString()} Hz / ${metrics.channels} ch`}
            />
            <Info label="Packet" value={`${metrics.packetFrames} frames`} />
            <Info
              label="Queue depth"
              value={`${metrics.queuedFrames} / ${metrics.capacityFrames}`}
            />
            <Info label="Latency estimate" value={`${estimatedLatency.toFixed(1)} ms`} />
            <Info label="Underruns" value={metrics.underrunFrames.toLocaleString()} mono />
            <Info label="Dropped frames" value={metrics.droppedFrames.toLocaleString()} mono />
          </dl>
          <p className="mt-4 text-[11px] leading-5 text-zinc-600">
            Latency is an algorithm/queue estimate, not a measured end-to-end loopback result.
          </p>
        </section>
      </div>

      <section className="mt-5 rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="text-sm font-semibold text-zinc-200">Virtual routing</h2>
            <p className="mt-1 text-xs text-zinc-600">
              Start audio processing first. Headphones are recommended to prevent feedback.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Action
              icon={<Broadcast size={15} />}
              label="Start Routing"
              onClick={virtualMic.start}
              disabled={
                virtualMic.busy || !running || !status.bridgeConnected || status.routingActive
              }
              primary
            />
            <Action
              label="Stop Routing"
              onClick={virtualMic.stop}
              disabled={virtualMic.busy || !status.routingActive}
            />
            <Action
              label="Test Microphone"
              onClick={virtualMic.openSoundSettings}
              disabled={!status.routingActive}
            />
            <Action
              icon={<ArrowSquareOut size={15} />}
              label="Open Windows Sound Settings"
              onClick={virtualMic.openSoundSettings}
            />
          </div>
        </div>
        {!running ? (
          <p className="mt-4 text-xs text-amber-200/70">
            Start Monitoring on Dashboard before starting virtual routing. The app never opens the
            microphone automatically.
          </p>
        ) : null}
      </section>
    </div>
  );
}

function Info({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <dt className="text-zinc-600">{label}</dt>
      <dd className={`mt-1 break-words text-zinc-300 ${mono ? "font-mono" : ""}`}>{value}</dd>
    </div>
  );
}

function Action({
  label,
  icon,
  onClick,
  disabled = false,
  primary = false,
}: {
  label: string;
  icon?: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  primary?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`inline-flex h-9 items-center gap-2 rounded-lg px-3 text-xs font-medium disabled:cursor-not-allowed disabled:opacity-35 ${primary ? "bg-cyan-300 text-[#071014]" : "border border-white/[0.09] text-zinc-300 hover:text-cyan-200"}`}
    >
      {icon}
      {label}
    </button>
  );
}
