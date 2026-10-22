import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../src/App";
import { VoiceStudio } from "../src/pages/VoiceStudio";
import type { AudioRuntimeStatus } from "../src/types/audio";

const audioMocks = vi.hoisted(() => ({
  refresh: vi.fn(),
  selectInput: vi.fn(),
  selectOutput: vi.fn(),
}));

const runtimeMocks = vi.hoisted(() => ({
  status: vi.fn(),
  start: vi.fn(),
  stop: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
}));
const volumeMocks = vi.hoisted(() => ({
  get: vi.fn(),
  preview: vi.fn(),
  set: vi.fn(),
}));
const noiseMocks = vi.hoisted(() => ({ get: vi.fn(), preview: vi.fn(), set: vi.fn() }));
const recordingMocks = vi.hoisted(() => ({
  status: vi.fn(),
  start: vi.fn(),
  stop: vi.fn(),
  play: vi.fn(),
  stopPlayback: vi.fn(),
  hear: vi.fn(),
  export: vi.fn(),
  remove: vi.fn(),
}));
const effectMocks = vi.hoisted(() => ({
  get: vi.fn(),
  set: vi.fn(),
  reset: vi.fn(),
}));
const presetMocks = vi.hoisted(() => ({
  list: vi.fn(),
  active: vi.fn(),
  apply: vi.fn(),
  reset: vi.fn(),
}));

const preset = (id: string, name: string, category = "Character") => ({
  schemaVersion: 1,
  id,
  name,
  description: `${name} description`,
  category,
  pitchSemitones: id === "deep" ? -4 : 0,
  formantSemitones: 0,
  gain: { makeupDb: 0 },
  effects: [],
  builtIn: true,
});
const builtInPresets = [
  preset("natural", "Natural Voice", "Natural"),
  preset("deep", "Deep Voice"),
  preset("high", "High Voice"),
  preset("masculine", "Masculine Voice"),
  preset("feminine", "Feminine Voice"),
  preset("robot", "Robot Voice", "Creative"),
  preset("radio", "Radio Voice", "Utility"),
  preset("cartoon", "Cartoon Voice", "Creative"),
];

const stoppedRuntime: AudioRuntimeStatus = {
  state: "stopped",
  inputDeviceId: null,
  outputDeviceId: null,
  sampleRate: null,
  inputChannels: null,
  outputChannels: null,
  dspLatencyFrames: null,
  metrics: null,
};
let currentRuntime: AudioRuntimeStatus = stoppedRuntime;

const connectedSnapshot = {
  inputDevices: [
    {
      id: "wasapi:microphone-1",
      name: "Studio Microphone",
      kind: "input",
      isDefault: true,
      sampleRate: 48000,
      channels: 2,
      sampleFormat: "f32",
      bufferCapability: { minimumFrames: 128, maximumFrames: 1024 },
      fingerprint: "studio-microphone|input|48000|2|f32",
    },
  ],
  outputDevices: [
    {
      id: "wasapi:speakers-1",
      name: "Desktop Speakers",
      kind: "output",
      isDefault: true,
      sampleRate: 48000,
      channels: 2,
      sampleFormat: "f32",
      bufferCapability: null,
      fingerprint: "desktop-speakers|output|48000|2|f32",
    },
  ],
  inputSelection: { selectedId: "wasapi:microphone-1", fellBack: false, message: null },
  outputSelection: { selectedId: "wasapi:speakers-1", fellBack: false, message: null },
};

vi.mock("../src/services/tauri", () => ({
  getAppStatus: vi.fn().mockResolvedValue({
    appName: "VoiceMorph Studio",
    version: "0.1.0",
    platform: "windows-x86_64",
    audioEngineReady: false,
  }),
  refreshAudioDevices: audioMocks.refresh,
  selectInputDevice: audioMocks.selectInput,
  selectOutputDevice: audioMocks.selectOutput,
  getAudioRuntimeStatus: runtimeMocks.status,
  startMonitoring: runtimeMocks.start,
  stopMonitoring: runtimeMocks.stop,
  pauseMonitoring: runtimeMocks.pause,
  resumeMonitoring: runtimeMocks.resume,
  getVolumeSettings: volumeMocks.get,
  previewVolumeSettings: volumeMocks.preview,
  setVolumeSettings: volumeMocks.set,
  getNoiseSettings: noiseMocks.get,
  previewNoiseSettings: noiseMocks.preview,
  setNoiseSettings: noiseMocks.set,
  getRecordingStatus: recordingMocks.status,
  startRecording: recordingMocks.start,
  stopRecording: recordingMocks.stop,
  playRecording: recordingMocks.play,
  stopPlayback: recordingMocks.stopPlayback,
  setHearMyself: recordingMocks.hear,
  exportRecording: recordingMocks.export,
  deleteRecording: recordingMocks.remove,
  getEffectParameters: effectMocks.get,
  setEffectParameters: effectMocks.set,
  resetEffectParameters: effectMocks.reset,
  getBuiltinPresets: presetMocks.list,
  getActivePresetId: presetMocks.active,
  applyBuiltinPreset: presetMocks.apply,
  resetToNaturalPreset: presetMocks.reset,
}));

describe("desktop navigation", () => {
  beforeEach(() => {
    audioMocks.refresh.mockReset().mockResolvedValue({
      inputDevices: [],
      outputDevices: [],
      inputSelection: { selectedId: null, fellBack: false, message: null },
      outputSelection: { selectedId: null, fellBack: false, message: null },
    });
    audioMocks.selectInput.mockReset().mockResolvedValue(connectedSnapshot);
    audioMocks.selectOutput.mockReset().mockResolvedValue(connectedSnapshot);
    currentRuntime = stoppedRuntime;
    runtimeMocks.status.mockReset().mockImplementation(async () => currentRuntime);
    runtimeMocks.stop.mockReset().mockImplementation(async () => {
      currentRuntime = stoppedRuntime;
      return currentRuntime;
    });
    runtimeMocks.pause.mockReset().mockResolvedValue({ ...stoppedRuntime, state: "paused" });
    runtimeMocks.resume.mockReset().mockResolvedValue({ ...stoppedRuntime, state: "running" });
    const runningRuntime: AudioRuntimeStatus = {
      ...stoppedRuntime,
      state: "running",
      inputDeviceId: "wasapi:microphone-1",
      outputDeviceId: "wasapi:speakers-1",
      sampleRate: 48000,
      inputChannels: 2,
      outputChannels: 2,
      dspLatencyFrames: 2048,
      metrics: {
        sampleRate: 48000,
        inputBufferFrames: 256,
        outputBufferFrames: 256,
        dspLatencyFrames: 2048,
        underrunCount: 0,
        overflowCount: 0,
        streamErrorCount: 0,
        averageProcessingMicros: 0.1,
        estimatedLatencyMs: 10.7,
        ringDepthFrames: 128,
        inputLevel: 0.25,
        outputLevel: 0.25,
        inputPeakDbfs: -12,
        inputRmsDbfs: -18,
        outputPeakDbfs: -12,
        outputRmsDbfs: -18,
        inputClipping: false,
        outputClipping: false,
        streamFaulted: false,
        protectedClipCount: 0,
      },
    };
    runtimeMocks.start.mockReset().mockImplementation(async () => {
      currentRuntime = runningRuntime;
      return currentRuntime;
    });
    const volumeSettings = {
      inputPercent: 100,
      outputPercent: 100,
      inputMuted: false,
      outputMuted: false,
    };
    volumeMocks.get.mockReset().mockResolvedValue(volumeSettings);
    volumeMocks.preview.mockReset().mockImplementation(async (settings) => settings);
    volumeMocks.set.mockReset().mockImplementation(async (settings) => settings);
    const noiseSettings = {
      enabled: false,
      bypassed: false,
      suppressionStrength: 0.8,
      gateEnabled: true,
      gateThresholdDbfs: -52,
      limiterEnabled: true,
      limiterCeilingDbfs: -1,
    };
    noiseMocks.get.mockReset().mockResolvedValue(noiseSettings);
    noiseMocks.preview.mockReset().mockImplementation(async (settings) => settings);
    noiseMocks.set.mockReset().mockImplementation(async (settings) => settings);
    const recordingStatus = {
      recording: false,
      playing: false,
      hearMyself: false,
      source: "processed",
      durationSeconds: 0,
      playbackSeconds: 0,
      droppedFrames: 0,
      hasRecording: false,
      incomplete: false,
      error: null,
    };
    for (const mock of Object.values(recordingMocks))
      mock.mockReset().mockResolvedValue(recordingStatus);
    const effectParameters = {
      pitchSemitones: 0,
      formantSemitones: 0,
      enabled: true,
      bypassed: false,
    };
    effectMocks.get.mockReset().mockResolvedValue(effectParameters);
    effectMocks.set.mockReset().mockImplementation(async (parameters) => parameters);
    effectMocks.reset.mockReset().mockResolvedValue(effectParameters);
    presetMocks.list.mockReset().mockResolvedValue(builtInPresets);
    presetMocks.active.mockReset().mockResolvedValue("natural");
    presetMocks.apply.mockReset().mockImplementation(async (id) => ({
      preset: builtInPresets.find((item) => item.id === id),
      parameters: effectParameters,
    }));
    presetMocks.reset.mockReset().mockResolvedValue({
      preset: builtInPresets[0],
      parameters: effectParameters,
    });
    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 900 });
    window.dispatchEvent(new Event("resize"));
  });

  it("navigates between every Task 02 page at the minimum window width", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(screen.getByRole("heading", { name: "Dashboard" })).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Voice Effects" })[0]);
    expect(screen.getByRole("heading", { name: "Voice Effects" })).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Voice Studio" })[0]);
    expect(screen.getByRole("heading", { name: "Voice Studio" })).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Settings" })[0]);
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });

  it("renders disconnected microphone state without fake live data", async () => {
    render(<App />);

    expect(await screen.findByRole("alert")).toHaveTextContent("Microphone is not connected");
    expect(screen.getByLabelText("Input device")).toBeDisabled();
    expect(screen.getByRole("meter", { name: "Input level" })).toHaveAttribute(
      "aria-valuenow",
      "-60",
    );
    expect(screen.getByRole("button", { name: "Start recording" })).toBeDisabled();
  });

  it("filters and applies a validated backend preset", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getAllByRole("button", { name: "Voice Effects" })[0]);
    await user.type(screen.getByLabelText("Search presets"), "Deep");

    expect(await screen.findByRole("button", { name: /Deep Voice/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Natural Voice/i })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Deep Voice/i }));
    await user.click(screen.getByRole("button", { name: "Apply preset" }));
    expect(presetMocks.apply).toHaveBeenCalledWith("deep");
    expect(await screen.findByRole("status")).toHaveTextContent("validated and applied");
  });

  it("renders backend devices and refreshes without opening an audio stream", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const user = userEvent.setup();
    render(<App />);

    expect(
      await screen.findByRole("option", { name: "Studio Microphone (Default)" }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/48,000 Hz.*2 channels.*f32/i)).toHaveLength(2);

    await user.click(screen.getByRole("button", { name: "Refresh devices" }));
    expect(audioMocks.refresh).toHaveBeenCalledTimes(2);
  });

  it("starts and stops monitoring only after an explicit user action", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const user = userEvent.setup();
    render(<App />);

    const start = await screen.findByRole("button", { name: "Start monitoring" });
    expect(runtimeMocks.start).not.toHaveBeenCalled();
    await user.click(start);
    expect(runtimeMocks.start).toHaveBeenCalledWith("wasapi:microphone-1", "wasapi:speakers-1");
    expect(await screen.findByRole("button", { name: "Stop monitoring" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Stop monitoring" }));
    expect(runtimeMocks.stop).toHaveBeenCalledOnce();
  });

  it("updates independent gain controls and microphone mute", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const user = userEvent.setup();
    render(<App />);

    const inputGain = await screen.findByLabelText("Microphone input gain");
    fireEvent.change(inputGain, { target: { value: "65" } });
    expect(volumeMocks.preview).toHaveBeenCalledWith(expect.objectContaining({ inputPercent: 65 }));

    await user.click(screen.getByRole("button", { name: "Mute microphone" }));
    expect(volumeMocks.set).toHaveBeenCalledWith(expect.objectContaining({ inputMuted: true }));
    expect(screen.getByLabelText("Output monitoring volume")).toHaveValue("100");
  });
});

describe("voice controls", () => {
  it("sends pitch, formant, bypass and reset controls to the DSP backend", async () => {
    const effectParameters = {
      pitchSemitones: 0,
      formantSemitones: 0,
      enabled: true,
      bypassed: false,
    };
    effectMocks.get.mockResolvedValue(effectParameters);
    effectMocks.set.mockImplementation(async (parameters) => parameters);
    effectMocks.reset.mockResolvedValue(effectParameters);
    runtimeMocks.status.mockResolvedValue(stoppedRuntime);
    const user = userEvent.setup();
    render(<VoiceStudio />);

    const pitch = await screen.findByLabelText("Pitch");
    fireEvent.change(pitch, { target: { value: "4" } });
    expect(pitch).toHaveValue("4");
    expect(effectMocks.set).toHaveBeenCalledWith(expect.objectContaining({ pitchSemitones: 4 }));

    const bypass = screen.getByRole("switch", { name: "Effect bypass" });
    expect(bypass).toHaveAttribute("aria-checked", "false");
    await user.click(bypass);
    expect(bypass).toHaveAttribute("aria-checked", "true");
    expect(effectMocks.set).toHaveBeenCalledWith(expect.objectContaining({ bypassed: true }));
    await user.click(screen.getByRole("button", { name: "Reset defaults" }));
    expect(effectMocks.reset).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: /Save preset/i })).toBeDisabled();
  });
});
