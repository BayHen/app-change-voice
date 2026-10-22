import { open, save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../services/tauri";
import type { VoicePresetSummary } from "../types/preset";

function message(reason: unknown) {
  return reason instanceof Error ? reason.message : String(reason);
}
export function usePresets() {
  const mounted = useRef(true);
  const [presets, setPresets] = useState<VoicePresetSummary[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [applyingId, setApplyingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const refresh = useCallback(async () => {
    const [items, active, warnings] = await Promise.all([
      api.getAllPresets(),
      api.getActivePresetId(),
      api.getPresetWarnings(),
    ]);
    if (mounted.current) {
      setPresets(items);
      setActiveId(active);
      if (warnings.length) setError(`Some preset files were skipped: ${warnings.join("; ")}`);
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    void Promise.resolve()
      .then(refresh)
      .catch((e) => setError(message(e)))
      .finally(() => setLoading(false));
    return () => {
      mounted.current = false;
    };
  }, [refresh]);
  const mutate = useCallback(
    async (operation: () => Promise<unknown>, success: string) => {
      setError(null);
      setNotice(null);
      try {
        await operation();
        await refresh();
        if (mounted.current) setNotice(success);
        return true;
      } catch (e) {
        if (mounted.current) setError(message(e));
        return false;
      }
    },
    [refresh],
  );
  const apply = useCallback(async (id: string) => {
    setApplyingId(id);
    setError(null);
    try {
      const result = await api.applyPreset(id);
      if (mounted.current) setActiveId(result.preset.id);
      return result.preset;
    } catch (e) {
      if (mounted.current) setError(message(e));
      return null;
    } finally {
      if (mounted.current) setApplyingId(null);
    }
  }, []);
  const reset = useCallback(async () => {
    const result = await api.resetToNaturalPreset();
    if (mounted.current) setActiveId(result.preset.id);
    return result.preset;
  }, []);
  const importJson = useCallback(async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Voice preset JSON", extensions: ["json"] }],
    });
    if (typeof path !== "string") return false;
    return mutate(() => api.importUserPreset(path), "Preset imported");
  }, [mutate]);
  const exportJson = useCallback(
    async (id: string) => {
      const path = await save({
        defaultPath: `${id}.json`,
        filters: [{ name: "Voice preset JSON", extensions: ["json"] }],
      });
      if (!path) return false;
      return mutate(() => api.exportUserPreset(id, path), "Preset exported");
    },
    [mutate],
  );
  return {
    presets,
    activeId,
    loading,
    applyingId,
    error,
    notice,
    apply,
    reset,
    create: (p: VoicePresetSummary) => mutate(() => api.createUserPreset(p), "Preset created"),
    update: (p: VoicePresetSummary) => mutate(() => api.updateUserPreset(p), "Preset saved"),
    duplicate: (id: string) => mutate(() => api.duplicatePreset(id), "Preset duplicated"),
    remove: (id: string) => mutate(() => api.deleteUserPreset(id), "Preset deleted"),
    favorite: (id: string, value: boolean) =>
      mutate(() => api.setPresetFavorite(id, value), "Favorite updated"),
    importJson,
    exportJson,
  };
}
