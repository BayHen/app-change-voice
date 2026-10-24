import { useCallback, useEffect, useRef, useState } from "react";
import {
  checkVirtualMicrophoneDriver,
  connectVirtualMicrophone,
  disconnectVirtualMicrophone,
  getVirtualMicrophoneMetrics,
  getVirtualMicrophoneStatus,
  openWindowsSoundSettings,
  startVirtualMicrophoneStream,
  stopVirtualMicrophoneStream,
} from "../services/tauri";
import type {
  VirtualMicrophoneCommandError,
  VirtualMicrophoneMetrics,
  VirtualMicrophoneStatus,
} from "../types/audio";

const emptyStatus: VirtualMicrophoneStatus = {
  state: "driverUnavailable",
  driverName: "VoiceMorph Virtual Microphone",
  driverVersion: null,
  expectedDriverVersion: "0.1.0.0",
  versionCompatible: false,
  protocolVersion: 1,
  captureEndpointAvailable: false,
  captureEndpointName: null,
  bridgeConnected: false,
  routingActive: false,
  message: "Checking VoiceMorph driver…",
};
const emptyMetrics: VirtualMicrophoneMetrics = {
  connected: false,
  streaming: false,
  sampleRate: 48000,
  channels: 1,
  packetFrames: 480,
  queuedFrames: 0,
  capacityFrames: 0,
  writtenFrames: 0,
  droppedFrames: 0,
  underrunFrames: 0,
  transportErrors: 0,
  bridgeBufferEstimateMs: 0,
};

function errorMessage(reason: unknown) {
  if (reason && typeof reason === "object" && "message" in reason)
    return (reason as VirtualMicrophoneCommandError).message;
  return reason instanceof Error ? reason.message : String(reason);
}

export function useVirtualMicrophone() {
  const mounted = useRef(true);
  const [status, setStatus] = useState(emptyStatus);
  const [metrics, setMetrics] = useState(emptyMetrics);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [nextStatus, nextMetrics] = await Promise.all([
        getVirtualMicrophoneStatus(),
        getVirtualMicrophoneMetrics(),
      ]);
      if (mounted.current) {
        setStatus(nextStatus);
        setMetrics(nextMetrics);
        setError(null);
      }
    } catch (reason) {
      if (mounted.current) setError(errorMessage(reason));
    } finally {
      if (mounted.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    const initial = window.setTimeout(() => void refresh(), 0);
    const interval = window.setInterval(() => void refresh(), 500);
    return () => {
      mounted.current = false;
      window.clearTimeout(initial);
      window.clearInterval(interval);
    };
  }, [refresh]);

  const run = useCallback(
    async (operation: () => Promise<unknown>) => {
      setBusy(true);
      setError(null);
      try {
        await operation();
        await refresh();
      } catch (reason) {
        if (mounted.current) setError(errorMessage(reason));
      } finally {
        if (mounted.current) setBusy(false);
      }
    },
    [refresh],
  );

  return {
    status,
    metrics,
    loading,
    busy,
    error,
    check: () => run(checkVirtualMicrophoneDriver),
    connect: () => run(connectVirtualMicrophone),
    disconnect: () => run(disconnectVirtualMicrophone),
    start: () => run(startVirtualMicrophoneStream),
    stop: () => run(stopVirtualMicrophoneStream),
    openSoundSettings: () => run(openWindowsSoundSettings),
  };
}
