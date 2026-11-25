import { createContext, useContext, type ReactNode } from "react";
import { useAudioRuntime } from "./useAudioRuntime";
import { useAudioDevices } from "./useAudioDevices";
import { useRecording } from "./useRecording";
import { useVirtualRouting } from "./useVirtualRouting";

type Session = {
  runtime: ReturnType<typeof useAudioRuntime>;
  devices: ReturnType<typeof useAudioDevices>;
  recording: ReturnType<typeof useRecording>;
  routing: ReturnType<typeof useVirtualRouting>;
};
const Context = createContext<Session | null>(null);
export function AudioSessionProvider({ children }: { children: ReactNode }) {
  const runtime = useAudioRuntime();
  const devices = useAudioDevices();
  const recording = useRecording();
  const routing = useVirtualRouting();
  return (
    <Context.Provider value={{ runtime, devices, recording, routing }}>{children}</Context.Provider>
  );
}
// The provider and its consumer intentionally share this typed context module.
// eslint-disable-next-line react-refresh/only-export-components
export function useAudioSession() {
  const session = useContext(Context);
  if (!session) throw new Error("Audio session provider is required");
  return session;
}
