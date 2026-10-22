import { DownloadSimple, Play, Record, Stop, Trash } from "@phosphor-icons/react";
import { useState } from "react";
import type { RecordingSource, RecordingStatus } from "../types/audio";
import { ToggleSwitch } from "./ToggleSwitch";

interface Props {
  status: RecordingStatus;
  busy: boolean;
  error: string | null;
  sessionActive: boolean;
  onStart: () => void;
  onStop: () => void;
  onPlay: (source: RecordingSource) => void;
  onStopPlayback: () => void;
  onHear: (enabled: boolean) => void;
  onExport: (source: RecordingSource) => void;
  onDelete: () => void;
}
const time = (seconds: number) =>
  `${Math.floor(seconds / 60)
    .toString()
    .padStart(2, "0")}:${Math.floor(seconds % 60)
    .toString()
    .padStart(2, "0")}`;
export function RecordingPanel({
  status,
  busy,
  error,
  sessionActive,
  onStart,
  onStop,
  onPlay,
  onStopPlayback,
  onHear,
  onExport,
  onDelete,
}: Props) {
  const [source, setSource] = useState<RecordingSource>("processed");
  return (
    <section
      className="mt-5 rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6"
      aria-label="Voice testing"
    >
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-sm font-semibold text-zinc-200">Voice testing</h2>
          <p className="mt-1 text-xs text-zinc-600">
            Local-only aligned original and processed WAV recording
          </p>
        </div>
        <span
          className={`rounded-full px-3 py-1 text-xs ${status.recording ? "bg-rose-400/10 text-rose-300" : "bg-white/[0.04] text-zinc-500"}`}
        >
          {status.recording ? "● Recording" : "Idle"} · {time(status.durationSeconds)}
        </span>
      </div>
      <div className="mt-5 grid gap-5 lg:grid-cols-[0.8fr_1.2fr]">
        <div className="space-y-4">
          <ToggleSwitch
            label="Hear Myself"
            description="Routes processed audio through the existing output stream. Use headphones."
            checked={status.hearMyself}
            disabled={!sessionActive || busy}
            onChange={onHear}
          />
          <p className="text-[11px] leading-5 text-zinc-600">
            Microphone capture starts only after Start monitoring and Record. Audio never leaves
            this device.
          </p>
        </div>
        <div>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={status.recording ? onStop : onStart}
              disabled={!sessionActive || busy}
              className="inline-flex h-10 items-center gap-2 rounded-xl bg-cyan-300/15 px-4 text-sm text-cyan-200 disabled:opacity-40"
            >
              {status.recording ? <Stop size={16} /> : <Record size={16} weight="fill" />}
              {status.recording ? "Stop recording" : "Start recording"}
            </button>
            <select
              aria-label="Playback source"
              value={source}
              disabled={!status.hasRecording || status.playing}
              onChange={(event) => setSource(event.target.value as RecordingSource)}
              className="h-10 rounded-xl border border-white/10 bg-[#090b0e] px-3 text-sm text-zinc-300"
            >
              <option value="processed">Processed</option>
              <option value="original">Original</option>
            </select>
            <button
              type="button"
              disabled={!status.hasRecording || busy || status.recording}
              onClick={status.playing ? onStopPlayback : () => onPlay(source)}
              className="inline-flex h-10 items-center gap-2 rounded-xl border border-white/10 px-4 text-sm text-zinc-300 disabled:opacity-40"
            >
              {status.playing ? <Stop size={15} /> : <Play size={15} />}{" "}
              {status.playing ? "Stop playback" : "Play"}
            </button>
            <button
              type="button"
              disabled={!status.hasRecording || busy}
              onClick={() => onExport(source)}
              className="inline-flex h-10 items-center gap-2 rounded-xl border border-white/10 px-4 text-sm text-zinc-300 disabled:opacity-40"
            >
              <DownloadSimple size={15} />
              Export WAV
            </button>
            <button
              type="button"
              disabled={!status.hasRecording || busy}
              onClick={onDelete}
              className="inline-flex h-10 items-center gap-2 rounded-xl border border-rose-300/10 px-3 text-sm text-rose-300 disabled:opacity-40"
            >
              <Trash size={15} />
              Delete
            </button>
          </div>
          <input
            aria-label="Playback timeline"
            className="mt-5 w-full accent-cyan-300"
            type="range"
            min="0"
            max={Math.max(status.durationSeconds, 0.01)}
            value={Math.min(status.playbackSeconds, status.durationSeconds)}
            readOnly
          />
          <div className="mt-1 flex justify-between font-mono text-[11px] text-zinc-600">
            <span>{time(status.playbackSeconds)}</span>
            <span>{time(status.durationSeconds)}</span>
          </div>
        </div>
      </div>
      {status.incomplete ? (
        <p role="alert" className="mt-4 text-xs text-amber-300">
          Recording queue overflowed: {status.droppedFrames} frames were dropped. This recording may
          be incomplete.
        </p>
      ) : null}
      {error || status.error ? (
        <p role="alert" className="mt-4 text-xs text-rose-300">
          {error ?? status.error}
        </p>
      ) : null}
    </section>
  );
}
