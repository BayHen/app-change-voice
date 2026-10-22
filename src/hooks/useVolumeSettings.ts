import { useCallback, useEffect, useRef, useState } from "react";
import { getVolumeSettings, previewVolumeSettings, setVolumeSettings } from "../services/tauri";
import type { VolumeSettings } from "../types/audio";

const defaults: VolumeSettings = {
  inputPercent: 100,
  outputPercent: 100,
  inputMuted: false,
  outputMuted: false,
};

export function useVolumeSettings() {
  const mounted = useRef(true);
  const [settings, setSettingsState] = useState(defaults);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    mounted.current = true;
    void getVolumeSettings()
      .then((value) => mounted.current && setSettingsState(value))
      .catch((reason) => mounted.current && setError(String(reason)))
      .finally(() => mounted.current && setLoading(false));
    return () => {
      mounted.current = false;
    };
  }, []);

  const preview = useCallback((next: VolumeSettings) => {
    setSettingsState(next);
    void previewVolumeSettings(next).catch((reason) => mounted.current && setError(String(reason)));
  }, []);

  const commit = useCallback(async (next: VolumeSettings) => {
    setSettingsState(next);
    try {
      const saved = await setVolumeSettings(next);
      if (mounted.current) setSettingsState(saved);
    } catch (reason) {
      if (mounted.current) setError(String(reason));
    }
  }, []);

  return { settings, loading, error, preview, commit };
}
