import { useCallback, useEffect, useRef, useState } from "react";
import { getEffectParameters, resetEffectParameters, setEffectParameters } from "../services/tauri";
import type { EffectParameters } from "../types/audio";

const defaults: EffectParameters = {
  pitchSemitones: 0,
  formantSemitones: 0,
  enabled: true,
  bypassed: false,
  makeupGainDb: 0,
  ringModFrequencyHz: 72,
  ringModMix: 0,
  radioHighPassHz: 280,
  radioLowPassHz: 3400,
  radioDrive: 1,
  radioMix: 0,
  tremoloRateHz: 5,
  tremoloDepth: 0,
  bassDb: 0,
  trebleDb: 0,
};

const message = (reason: unknown) => (reason instanceof Error ? reason.message : String(reason));

export function useEffectParameters() {
  const mounted = useRef(true);
  const [parameters, setLocalParameters] = useState(defaults);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    mounted.current = true;
    void getEffectParameters()
      .then((value) => {
        if (mounted.current) setLocalParameters(value);
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

  const update = useCallback(async (next: EffectParameters) => {
    setLocalParameters(next);
    setError(null);
    try {
      const validated = await setEffectParameters(next);
      if (mounted.current) setLocalParameters(validated);
    } catch (reason) {
      if (mounted.current) setError(message(reason));
    }
  }, []);

  const reset = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const value = await resetEffectParameters();
      if (mounted.current) setLocalParameters(value);
    } catch (reason) {
      if (mounted.current) setError(message(reason));
    } finally {
      if (mounted.current) setLoading(false);
    }
  }, []);

  return { parameters, loading, error, update, reset };
}
