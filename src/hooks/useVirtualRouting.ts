import { useCallback, useEffect, useRef, useState } from "react";
import {
  getRoutingConfiguration,
  getRoutingStatus,
  getVirtualOutputDevices,
  openWindowsSoundSettings,
  selectVirtualOutputDevice,
  startVirtualRouting,
  stopVirtualRouting,
} from "../services/tauri";
import type { RoutingConfiguration, RoutingStatus, VirtualOutputDevice } from "../types/audio";

const emptyStatus: RoutingStatus = {
  state: "notConfigured",
  outputDeviceId: null,
  outputDeviceName: null,
  sampleRate: null,
  channels: null,
  underrunCount: 0,
  overflowCount: 0,
  outputPeakDbfs: -96,
  endpointVerified: false,
  message: null,
};

const message = (error: unknown) =>
  error instanceof Error
    ? error.message
    : typeof error === "object" && error && "message" in error
      ? String(error.message)
      : String(error);

export function useVirtualRouting() {
  const mounted = useRef(true);
  const [devices, setDevices] = useState<VirtualOutputDevice[]>([]);
  const [configuration, setConfiguration] = useState<RoutingConfiguration>({
    outputDeviceId: null,
    outputDeviceFingerprint: null,
  });
  const [status, setStatus] = useState(emptyStatus);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [nextDevices, nextConfiguration, nextStatus] = await Promise.all([
        getVirtualOutputDevices(),
        getRoutingConfiguration(),
        getRoutingStatus(),
      ]);
      if (mounted.current) {
        setDevices(nextDevices);
        setConfiguration(nextConfiguration);
        setStatus(nextStatus);
        setError(null);
      }
    } catch (reason) {
      if (mounted.current) setError(message(reason));
    } finally {
      if (mounted.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    const initial = window.setTimeout(() => void refresh(), 0);
    const interval = window.setInterval(
      () =>
        void getRoutingStatus()
          .then((next) => mounted.current && setStatus(next))
          .catch((reason) => mounted.current && setError(message(reason))),
      100,
    );
    return () => {
      mounted.current = false;
      window.clearTimeout(initial);
      window.clearInterval(interval);
    };
  }, [refresh]);

  const run = useCallback(async (operation: () => Promise<RoutingStatus>) => {
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

  const select = useCallback(async (id: string) => {
    setBusy(true);
    setError(null);
    try {
      const next = await selectVirtualOutputDevice(id);
      if (mounted.current) {
        setConfiguration(next);
        setStatus(emptyStatus);
      }
    } catch (reason) {
      if (mounted.current) setError(message(reason));
    } finally {
      if (mounted.current) setBusy(false);
    }
  }, []);

  return {
    devices,
    configuration,
    status,
    loading,
    busy,
    error,
    refresh,
    select,
    start: () => run(startVirtualRouting),
    stop: () => run(stopVirtualRouting),
    openSoundSettings: () =>
      openWindowsSoundSettings().catch((reason) => setError(message(reason))),
  };
}
