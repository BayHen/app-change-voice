import { useCallback, useEffect, useRef, useState } from "react";
import {
  getAudioRuntimeStatus,
  pauseMonitoring,
  resumeMonitoring,
  startMonitoring,
  stopMonitoring,
} from "../services/tauri";
import type { AudioRuntimeStatus } from "../types/audio";

const stoppedStatus: AudioRuntimeStatus = {
  state: "stopped",
  inputDeviceId: null,
  outputDeviceId: null,
  sampleRate: null,
  inputChannels: null,
  outputChannels: null,
  metrics: null,
};

function applyRuntimeStatus(
  next: AudioRuntimeStatus,
  setStatus: (status: AudioRuntimeStatus) => void,
  setError: (error: string | null) => void,
) {
  setStatus(next);
  if (next.metrics?.streamFaulted) {
    setError("The audio driver reported a stream failure. Stop monitoring and refresh devices.");
  }
}

function message(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) return String(error.message);
  return String(error);
}

export function useAudioRuntime() {
  const mounted = useRef(true);
  const [status, setStatus] = useState(stoppedStatus);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async (operation: () => Promise<AudioRuntimeStatus>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await operation();
      if (mounted.current) applyRuntimeStatus(next, setStatus, setError);
      return next;
    } catch (reason) {
      if (mounted.current) setError(message(reason));
      return null;
    } finally {
      if (mounted.current) setBusy(false);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void getAudioRuntimeStatus().then(
      (next) => mounted.current && applyRuntimeStatus(next, setStatus, setError),
    );
    const interval = window.setInterval(() => {
      void getAudioRuntimeStatus()
        .then((next) => mounted.current && applyRuntimeStatus(next, setStatus, setError))
        .catch((reason) => mounted.current && setError(message(reason)));
    }, 40);
    return () => {
      mounted.current = false;
      window.clearInterval(interval);
    };
  }, []);

  return {
    status,
    busy,
    error,
    start: (inputId: string, outputId: string) => run(() => startMonitoring(inputId, outputId)),
    stop: () => run(stopMonitoring),
    pause: () => run(pauseMonitoring),
    resume: () => run(resumeMonitoring),
  };
}
