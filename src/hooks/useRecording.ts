import { save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../services/tauri";
import type { RecordingSource, RecordingStatus } from "../types/audio";

const empty: RecordingStatus = {
  recording: false,
  playing: false,
  hearMyself: false,
  source: "processed",
  durationSeconds: 0,
  playbackSeconds: 0,
  droppedFrames: 0,
  hasRecording: false,
  incomplete: false,
  error: null,
};
const message = (error: unknown) => (error instanceof Error ? error.message : String(error));
export function useRecording() {
  const mounted = useRef(true);
  const [status, setStatus] = useState(empty);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = useCallback(async (operation: () => Promise<RecordingStatus>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await operation();
      if (mounted.current) setStatus(next);
    } catch (reason) {
      if (mounted.current) setError(message(reason));
    } finally {
      if (mounted.current) setBusy(false);
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    const refresh = () =>
      void api
        .getRecordingStatus()
        .then((value) => mounted.current && setStatus(value))
        .catch(() => undefined);
    refresh();
    const timer = window.setInterval(refresh, 100);
    return () => {
      mounted.current = false;
      window.clearInterval(timer);
    };
  }, []);
  const exportWav = useCallback(
    async (source: RecordingSource) => {
      const path = await save({
        defaultPath: `voicemorph-${source}.wav`,
        filters: [{ name: "WAV audio", extensions: ["wav"] }],
      });
      if (path) await run(() => api.exportRecording(source, path));
    },
    [run],
  );
  return {
    status,
    busy,
    error,
    start: () => run(api.startRecording),
    stop: () => run(api.stopRecording),
    play: (source: RecordingSource) => run(() => api.playRecording(source)),
    stopPlayback: () => run(api.stopPlayback),
    setHear: (enabled: boolean) => run(() => api.setHearMyself(enabled)),
    remove: () => run(api.deleteRecording),
    exportWav,
  };
}
