import { useCallback, useEffect, useRef, useState } from "react";
import { getNoiseSettings, previewNoiseSettings, setNoiseSettings } from "../services/tauri";
import type { NoiseSettings } from "../types/audio";

const defaults: NoiseSettings = {
  enabled: false,
  bypassed: false,
  suppressionStrength: 0.8,
  gateEnabled: true,
  gateThresholdDbfs: -52,
  limiterEnabled: true,
  limiterCeilingDbfs: -1,
};
export function useNoiseSettings() {
  const mounted = useRef(true);
  const [settings, setSettings] = useState(defaults);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    mounted.current = true;
    void getNoiseSettings()
      .then((v) => mounted.current && setSettings(v))
      .catch((e) => mounted.current && setError(String(e)))
      .finally(() => mounted.current && setLoading(false));
    return () => {
      mounted.current = false;
    };
  }, []);
  const preview = useCallback((next: NoiseSettings) => {
    setSettings(next);
    void previewNoiseSettings(next).catch((e) => mounted.current && setError(String(e)));
  }, []);
  const commit = useCallback(async (next: NoiseSettings) => {
    setSettings(next);
    try {
      const saved = await setNoiseSettings(next);
      if (mounted.current) setSettings(saved);
    } catch (e) {
      if (mounted.current) setError(String(e));
    }
  }, []);
  return { settings, loading, error, preview, commit };
}
