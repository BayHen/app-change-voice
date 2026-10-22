import { useCallback, useEffect, useRef, useState } from "react";
import {
  applyBuiltinPreset,
  getActivePresetId,
  getBuiltinPresets,
  resetToNaturalPreset,
} from "../services/tauri";
import type { VoicePresetSummary } from "../types/preset";

function message(reason: unknown) {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "object" && reason && "message" in reason) return String(reason.message);
  return String(reason);
}

export function usePresets() {
  const mounted = useRef(true);
  const [presets, setPresets] = useState<VoicePresetSummary[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [applyingId, setApplyingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    mounted.current = true;
    void Promise.all([getBuiltinPresets(), getActivePresetId()])
      .then(([items, active]) => {
        if (mounted.current) {
          setPresets(items);
          setActiveId(active);
        }
      })
      .catch((reason) => {
        if (mounted.current) setError(message(reason));
      })
      .finally(() => {
        if (mounted.current) setLoading(false);
      });
    return () => {
      mounted.current = false;
    };
  }, []);

  const apply = useCallback(async (id: string) => {
    setApplyingId(id);
    setError(null);
    try {
      const result = await applyBuiltinPreset(id);
      if (mounted.current) setActiveId(result.preset.id);
      return result.preset;
    } catch (reason) {
      if (mounted.current) setError(message(reason));
      return null;
    } finally {
      if (mounted.current) setApplyingId(null);
    }
  }, []);

  const reset = useCallback(async () => {
    setApplyingId("natural");
    setError(null);
    try {
      const result = await resetToNaturalPreset();
      if (mounted.current) setActiveId(result.preset.id);
      return result.preset;
    } catch (reason) {
      if (mounted.current) setError(message(reason));
      return null;
    } finally {
      if (mounted.current) setApplyingId(null);
    }
  }, []);

  return { presets, activeId, loading, applyingId, error, apply, reset };
}
