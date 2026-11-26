import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { useAudioRuntime } from "../src/hooks/useAudioRuntime";
import type { AudioRuntimeStatus } from "../src/types/audio";

const api = vi.hoisted(() => ({ status: vi.fn(), start: vi.fn(), stop: vi.fn() }));
vi.mock("../src/services/tauri", () => ({
  getAudioRuntimeStatus: api.status,
  startMonitoring: api.start,
  stopMonitoring: api.stop,
  pauseMonitoring: vi.fn(),
  resumeMonitoring: vi.fn(),
}));

it("retains actual driver diagnostics until explicit stop, without restarting capture", async () => {
  const stopped: AudioRuntimeStatus = {
    state: "stopped",
    inputDeviceId: null,
    outputDeviceId: null,
    sampleRate: null,
    inputChannels: null,
    outputChannels: null,
    dspLatencyFrames: null,
    metrics: null,
  };
  const fault: AudioRuntimeStatus = {
    ...stopped,
    state: "faulted",
    metrics: {
      sampleRate: 48000,
      inputBufferFrames: 480,
      outputBufferFrames: 480,
      dspLatencyFrames: 2048,
      underrunCount: 0,
      overflowCount: 0,
      streamErrorCount: 1,
      lastStreamError: "Audio output: device disconnected",
      averageProcessingMicros: 0,
      estimatedLatencyMs: 0,
      ringDepthFrames: 0,
      inputLevel: 0,
      outputLevel: 0,
      inputPeakDbfs: -120,
      inputRmsDbfs: -120,
      outputPeakDbfs: -120,
      outputRmsDbfs: -120,
      inputClipping: false,
      outputClipping: false,
      streamFaulted: true,
      protectedClipCount: 0,
    },
  };
  api.status.mockResolvedValue(fault);
  api.stop.mockImplementation(async () => {
    api.status.mockResolvedValue(stopped);
    return stopped;
  });
  const { result, unmount } = renderHook(() => useAudioRuntime());
  await waitFor(() => expect(result.current.status.state).toBe("faulted"));
  expect(result.current.error).toContain("Audio output: device disconnected");
  expect(api.start).not.toHaveBeenCalled();
  await act(async () => {
    await result.current.stop();
  });
  expect(result.current.status.state).toBe("stopped");
  expect(result.current.error).toBeNull();
  expect(api.start).not.toHaveBeenCalled();
  unmount();
});
