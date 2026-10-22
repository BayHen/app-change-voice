import { useEffect, useState } from "react";
import { getAppStatus } from "../services/tauri";
import type { AppStatus } from "../types/app";

interface AppStatusState {
  status: AppStatus | null;
  error: string | null;
  loading: boolean;
}

export function useAppStatus(): AppStatusState {
  const [state, setState] = useState<AppStatusState>({
    status: null,
    error: null,
    loading: true,
  });

  useEffect(() => {
    let active = true;

    getAppStatus()
      .then((status) => active && setState({ status, error: null, loading: false }))
      .catch((error: unknown) => {
        if (active) {
          const message = error instanceof Error ? error.message : String(error);
          setState({ status: null, error: message, loading: false });
        }
      });

    return () => {
      active = false;
    };
  }, []);

  return state;
}
