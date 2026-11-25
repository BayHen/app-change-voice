import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../services/tauri";
import type { AIEngineStatus, AIEngineConfig } from "../types/ai";

export function useAIEngine() {
  const [status, setStatus] = useState<AIEngineStatus | null>(null);
  const [busy, setBusy] = useState(false),
    [error, setError] = useState<string | null>(null);
  const [bridgeError, setBridgeError] = useState<string | null>(null);
  const mounted = useRef(true);
  const refresh = useCallback(async () => {
    try {
      const next = await api.getAIEngineStatus();
      if (mounted.current) {
        setStatus(next);
        setBridgeError(null);
      }
    } catch (reason) {
      if (mounted.current) setBridgeError(String(reason));
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    void Promise.resolve().then(refresh);
    const timer = window.setInterval(() => void refresh(), 500);
    return () => {
      mounted.current = false;
      window.clearInterval(timer);
    };
  }, [refresh]);
  const run = useCallback(async (operation: () => Promise<AIEngineStatus>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await operation();
      if (mounted.current) setStatus(next);
      return next;
    } catch (reason) {
      if (mounted.current) setError(String(reason));
      return null;
    } finally {
      if (mounted.current) setBusy(false);
    }
  }, []);
  return {
    status,
    busy,
    error: error ?? bridgeError,
    refresh,
    start: (modelId: string) => run(() => api.startAIConversion(modelId)),
    stop: () => run(api.stopAIConversion),
    classic: () => run(api.selectClassicVoice),
    save: (config: AIEngineConfig) => run(() => api.setAIEngineConfig(config)),
    install: (source: string, digest: string, trusted: boolean) =>
      run(() => api.installAIRuntime(source, digest, trusted)),
    rollback: () => run(api.rollbackAIRuntime),
    cancelInstall: () =>
      api.cancelAIRuntimeInstall().catch((reason) => {
        if (mounted.current) setError(String(reason));
      }),
  };
}
