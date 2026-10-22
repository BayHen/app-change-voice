import { useCallback, useEffect, useRef, useState } from "react";
import { refreshAudioDevices, selectInputDevice, selectOutputDevice } from "../services/tauri";
import type { AudioDeviceSnapshot, DeviceKind } from "../types/audio";

interface AudioDevicesState {
  snapshot: AudioDeviceSnapshot | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) return String(error.message);
  return String(error);
}

export function useAudioDevices() {
  const mounted = useRef(true);
  const [state, setState] = useState<AudioDevicesState>({
    snapshot: null,
    loading: true,
    refreshing: false,
    error: null,
  });

  const refresh = useCallback(async (initial = false, silent = false) => {
    setState((current) => ({
      ...current,
      loading: initial,
      refreshing: !initial && !silent,
      error: null,
    }));
    try {
      const snapshot = await refreshAudioDevices();
      if (!mounted.current) return;
      setState({ snapshot, loading: false, refreshing: false, error: null });
    } catch (error) {
      if (!mounted.current) return;
      setState((current) => ({
        ...current,
        loading: false,
        refreshing: false,
        error: errorMessage(error),
      }));
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh(true);
    const interval = window.setInterval(() => void refresh(false, true), 5000);
    return () => {
      mounted.current = false;
      window.clearInterval(interval);
    };
  }, [refresh]);

  const select = useCallback(async (kind: DeviceKind, id: string) => {
    setState((current) => ({ ...current, refreshing: true, error: null }));
    try {
      const snapshot =
        kind === "input" ? await selectInputDevice(id) : await selectOutputDevice(id);
      if (!mounted.current) return;
      setState({ snapshot, loading: false, refreshing: false, error: null });
    } catch (error) {
      if (!mounted.current) return;
      setState((current) => ({
        ...current,
        refreshing: false,
        error: errorMessage(error),
      }));
    }
  }, []);

  return { ...state, refresh: () => refresh(false), select };
}
