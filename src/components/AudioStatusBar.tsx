import { Headphones, Circle, Mic, MoreHorizontal, Square } from "lucide-react";
import { useAudioSession } from "../hooks/AudioSession";
import { Button, Badge } from "./ui";
import { ToggleSwitch } from "./ToggleSwitch";

export function AudioStatusBar({ onSettings }: { onSettings: () => void }) {
  const { runtime, devices, recording, routing, ai } = useAudioSession();
  const aiSelected = ai.status?.mode === "wOkada";
  const classicFallback = aiSelected && ai.status?.classicFallbackActive;
  const aiMuted =
    aiSelected && !classicFallback && !["ready", "converting"].includes(ai.status?.state ?? "");
  const live = runtime.status.state === "running";
  const metrics = live ? runtime.status.metrics : null;
  // Matches Rust diagnostics: processing time is measured per frame, not per buffer.
  const cpu =
    metrics && metrics.sampleRate > 0
      ? (metrics.averageProcessingMicros * metrics.sampleRate) / 10_000
      : null;
  const input = devices.snapshot?.inputSelection.selectedId,
    output = devices.snapshot?.outputSelection.selectedId;
  return (
    <footer className="vm-statusbar" aria-label="Audio status">
      <div className="vm-engine">
        <span className={`vm-dot ${live ? "is-live" : ""}`} />
        <strong>
          {devices.error
            ? "Desktop bridge offline"
            : live
              ? classicFallback
                ? "Classic DSP fallback active"
                : aiMuted
                  ? "AI voice muted"
                  : aiSelected
                    ? "AI voice selected"
                    : "Audio engine running"
              : runtime.status.state === "faulted"
                ? "Audio engine stopped after error"
                : "Audio engine idle"}
        </strong>
        <span className="vm-status-detail">
          {aiSelected
            ? ai.status?.roundTripMs == null
              ? "AI latency —"
              : `${ai.status.roundTripMs.toFixed(1)} ms AI round trip`
            : metrics
              ? `${metrics.estimatedLatencyMs.toFixed(1)} ms latency`
              : "Latency —"}
        </span>
        <span className="vm-status-detail">
          {cpu === null ? "DSP CPU —" : `DSP CPU ${cpu.toFixed(1)}%`}
        </span>
      </div>
      <div className="vm-status-actions">
        {runtime.status.diagnosticRecording && (
          <Badge tone="warning">Local diagnostic recording</Badge>
        )}
        <Headphones size={17} />
        <ToggleSwitch
          label="Monitor"
          checked={live}
          disabled={runtime.busy || !!devices.error || !input || !output}
          onChange={(enabled) => {
            if (enabled && input && output) void runtime.start(input, output);
            else void runtime.stop();
          }}
        />
        <Button
          variant={recording.status.recording ? "danger" : "secondary"}
          disabled={recording.busy || !live}
          onClick={() => void (recording.status.recording ? recording.stop() : recording.start())}
        >
          {recording.status.recording ? <Square size={14} /> : <Circle size={14} />}
          <span>{recording.status.recording ? "Stop recording" : "Record"}</span>
        </Button>
        <Badge tone={routing.status.state === "active" ? "success" : "neutral"}>
          <Mic size={13} /> Virtual mic: {routing.status.state === "active" ? "live" : "idle"}
        </Badge>
        <Button variant="ghost" aria-label="Audio options" onClick={onSettings}>
          <MoreHorizontal size={18} />
        </Button>
      </div>
      {!devices.error && (runtime.error || recording.error) ? (
        <span role="alert" className="vm-status-error">
          {runtime.error ?? recording.error}
        </span>
      ) : null}
    </footer>
  );
}
