import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../src/App";
import { VoiceStudio } from "../src/pages/VoiceStudio";
import { UpdateSettings } from "../src/components/UpdateSettings";
import type { AudioRuntimeStatus } from "../src/types/audio";
import femalePresets from "../src-tauri/presets/female.json";

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
const qualityMocks = vi.hoisted(() => ({
  get: vi.fn(),
  set: vi.fn(),
  capture: vi.fn(),
  start: vi.fn(),
  stop: vi.fn(),
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
  create: vi.fn(),
  list: vi.fn(),
  active: vi.fn(),
  apply: vi.fn(),
  reset: vi.fn(),
}));
const routingMocks = vi.hoisted(() => ({
  devices: vi.fn(),
  configuration: vi.fn(),
  status: vi.fn(),
  select: vi.fn(),
  start: vi.fn(),
  stop: vi.fn(),
  openSettings: vi.fn(),
}));
const hotkeyMocks = vi.hoisted(() => ({ get: vi.fn(), set: vi.fn(), reset: vi.fn() }));
const soundboardMocks = vi.hoisted(() => ({ list: vi.fn(), plugins: vi.fn(), effects: vi.fn() }));
const profileMocks = vi.hoisted(() => ({
  list: vi.fn(),
  save: vi.fn(),
  remove: vi.fn(),
  apply: vi.fn(),
}));
const backupMocks = vi.hoisted(() => ({ status: vi.fn() }));
const diagnosticsMocks = vi.hoisted(() => ({ get: vi.fn(), export: vi.fn() }));
const previewMocks = vi.hoisted(() => ({ render: vi.fn(), open: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: previewMocks.open,
  save: vi.fn().mockResolvedValue(null),
}));
const updaterMocks = vi.hoisted(() => ({
  configuration: vi.fn(),
  check: vi.fn(),
  relaunch: vi.fn(),
}));
const aiMocks = vi.hoisted(() => ({
  recommended: vi.fn(),
  official: vi.fn(),
  cancel: vi.fn(),
  engine: vi.fn(),
  startConversion: vi.fn(),
  stopConversion: vi.fn(),
  classic: vi.fn(),
  saveConfig: vi.fn(),
  models: vi.fn(),
  status: vi.fn(),
  add: vi.fn(),
  remove: vi.fn(),
  enable: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(vi.fn()) }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: updaterMocks.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: updaterMocks.relaunch }));

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
  getAudioQuality: qualityMocks.get,
  setAudioQuality: qualityMocks.set,
  getStageCaptureStatus: qualityMocks.capture,
  startStageCapture: qualityMocks.start,
  stopStageCapture: qualityMocks.stop,
  getAudioDiagnostics: diagnosticsMocks.get,
  exportAudioDiagnostics: diagnosticsMocks.export,
  getUpdateConfiguration: updaterMocks.configuration,
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
  getAllPresets: presetMocks.list,
  renderVoicePreview: previewMocks.render,
  renderAIVoicePreview: previewMocks.render,
  createUserPreset: presetMocks.create,
  getPresetWarnings: vi.fn().mockResolvedValue([]),
  getActivePresetId: presetMocks.active,
  applyBuiltinPreset: presetMocks.apply,
  applyPreset: presetMocks.apply,
  resetToNaturalPreset: presetMocks.reset,
  getVirtualOutputDevices: routingMocks.devices,
  getRoutingConfiguration: routingMocks.configuration,
  getRoutingStatus: routingMocks.status,
  selectVirtualOutputDevice: routingMocks.select,
  startVirtualRouting: routingMocks.start,
  stopVirtualRouting: routingMocks.stop,
  openWindowsSoundSettings: routingMocks.openSettings,
  getHotkeyConfiguration: hotkeyMocks.get,
  setHotkeyConfiguration: hotkeyMocks.set,
  resetHotkeyConfiguration: hotkeyMocks.reset,
  getSoundboardSounds: soundboardMocks.list,
  getPlugins: soundboardMocks.plugins,
  getRegisteredEffects: soundboardMocks.effects,
  addSound: vi.fn(),
  updateSound: vi.fn(),
  removeSound: vi.fn(),
  playSound: vi.fn(),
  stopSound: vi.fn(),
  setPluginEnabled: vi.fn(),
  getApplicationProfiles: profileMocks.list,
  saveApplicationProfile: profileMocks.save,
  deleteApplicationProfile: profileMocks.remove,
  applyApplicationProfile: profileMocks.apply,
  getSyncStatus: backupMocks.status,
  exportAllSettings: vi.fn(),
  previewBackup: vi.fn(),
  restoreBackup: vi.fn(),
  exportAllPresets: vi.fn(),
  importAllPresets: vi.fn(),
  getAIModels: aiMocks.models,
  getAIEngineStatus: aiMocks.engine,
  detectAIHardware: vi.fn(),
  installRecommendedAIRuntime: aiMocks.recommended,
  installOfficialAIVoice: aiMocks.official,
  startAIConversion: aiMocks.startConversion,
  stopAIConversion: aiMocks.stopConversion,
  selectClassicVoice: aiMocks.classic,
  setAIEngineConfig: aiMocks.saveConfig,
  installAIRuntime: vi.fn(),
  cancelAIRuntimeInstall: aiMocks.cancel,
  rollbackAIRuntime: vi.fn(),
  getAIProviderStatus: aiMocks.status,
  addAIModel: aiMocks.add,
  removeAIModel: aiMocks.remove,
  setAIModelEnabled: aiMocks.enable,
}));

describe("desktop navigation", () => {
  it("downloads the runtime only after an explicit user action", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "AI Voice Studio" }));
    const install = await screen.findByRole("button", { name: "Install recommended runtime" });
    expect(aiMocks.recommended).not.toHaveBeenCalled();
    await user.click(install);
    expect(aiMocks.recommended).toHaveBeenCalledOnce();
    expect(aiMocks.startConversion).not.toHaveBeenCalled();
    expect(runtimeMocks.start).not.toHaveBeenCalled();
  });
  it("shows real download fields and exposes cancellation", async () => {
    const status = await aiMocks.engine();
    aiMocks.engine.mockResolvedValue({
      ...status,
      install: { ...status.install, installing: true },
      download: {
        phase: "downloading",
        component: "Official component",
        completedBytes: 104857600,
        totalBytes: 209715200,
        completedComponents: 1,
        totalComponents: 2,
      },
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "AI Voice Studio" }));
    const progress = await screen.findByRole("progressbar", { name: "AI runtime download" });
    expect(progress).toHaveAttribute("value", "104857600");
    expect(progress).toHaveAttribute("max", "209715200");
    await user.click(screen.getByRole("button", { name: "Cancel install" }));
    expect(aiMocks.cancel).toHaveBeenCalledOnce();
  });
  it("requires original-owner terms before preparing the official voice", async () => {
    const status = await aiMocks.engine();
    aiMocks.engine.mockResolvedValue({ ...status, runtimeInstalled: true });
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "AI Voice Studio" }));
    await user.click(
      await screen.findByRole("button", { name: "Get official Tsukuyomi-chan voice" }),
    );
    const download = screen.getByRole("button", { name: "Download & prepare official voice" });
    expect(download).toBeDisabled();
    expect(aiMocks.official).not.toHaveBeenCalled();
    await user.click(screen.getByRole("checkbox", { name: /I have read the owner/ }));
    await user.click(download);
    expect(aiMocks.official).toHaveBeenCalledWith(true);
    expect(aiMocks.startConversion).not.toHaveBeenCalled();
  });
  it("keeps AI opt-in, blocks loading without runtime and saves explicit failure policy", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "AI Voice Studio" }));
    const compute = await screen.findByLabelText("AI compute preference");
    expect(screen.getByRole("button", { name: "Load & start AI" })).toBeDisabled();
    expect(aiMocks.startConversion).not.toHaveBeenCalled();
    await user.selectOptions(compute, "cpu");
    await user.click(screen.getByRole("button", { name: "Apply AI settings" }));
    expect(aiMocks.saveConfig).toHaveBeenCalledWith(
      expect.objectContaining({ provider: "cpu", failurePolicy: "mute", allowCpuFallback: false }),
    );
    await user.click(screen.getByRole("button", { name: "Use Classic DSP" }));
    expect(aiMocks.classic).toHaveBeenCalledOnce();
  });
  it("allows neural file inspection before rights acknowledgement", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "AI Voice Studio" }));
    await user.click(screen.getByRole("button", { name: "Import ONNX model" }));
    expect(screen.getByRole("button", { name: "Choose ONNX or JSON" })).toBeEnabled();
    expect(aiMocks.add).not.toHaveBeenCalled();
  });
  beforeEach(() => {
    const capture = { running: false, folder: null, files: [], droppedSamples: 0, message: null };
    qualityMocks.get.mockReset().mockResolvedValue({ profile: "balanced" });
    qualityMocks.set.mockReset().mockImplementation(async (settings) => settings);
    qualityMocks.capture.mockReset().mockResolvedValue(capture);
    qualityMocks.start.mockReset().mockImplementation(async () => {
      const active = { ...capture, running: true, folder: "test-local-capture" };
      qualityMocks.capture.mockResolvedValue(active);
      return active;
    });
    qualityMocks.stop.mockReset().mockResolvedValue(capture);
    const engine = {
      state: "notInstalled",
      mode: "classicDsp",
      classicFallbackActive: false,
      hardware: null,
      runtimeVersion: null,
      recommendedRuntimeVersion: "test-recipe",
      recommendedDownloadBytes: 4184518137,
      download: null,
      runtimeInstalled: false,
      modelId: null,
      provider: null,
      upstreamCommit: "test-fixture",
      loadingMs: null,
      conversionMs: null,
      roundTripMs: null,
      queuedInputSamples: 0,
      queuedOutputSamples: 0,
      droppedInputSamples: 0,
      droppedOutputSamples: 0,
      missingOutputSamples: 0,
      completedChunks: 0,
      restarts: 0,
      message: null,
      config: {
        provider: "auto",
        allowCpuFallback: false,
        failurePolicy: "mute",
        pitchSemitones: 0,
        f0Method: "dio",
        chunkFrames: 4096,
        crossfadeFrames: 1024,
        extraContextFrames: 4096,
        noiseThreshold: 0.00001,
        cpuThreads: 4,
      },
      install: {
        installed: false,
        installing: false,
        copiedFiles: 0,
        totalFiles: 0,
        version: null,
        message: null,
      },
    };
    aiMocks.engine.mockReset().mockResolvedValue(engine);
    aiMocks.recommended.mockReset().mockResolvedValue({
      ...engine,
      runtimeInstalled: true,
      runtimeVersion: engine.recommendedRuntimeVersion,
    });
    aiMocks.official.mockReset().mockResolvedValue({ ...engine, runtimeInstalled: true });
    aiMocks.cancel.mockReset().mockResolvedValue(undefined);
    aiMocks.startConversion.mockReset();
    aiMocks.stopConversion
      .mockReset()
      .mockResolvedValue({ ...engine, mode: "wOkada", state: "stopped" });
    aiMocks.classic.mockReset().mockResolvedValue(engine);
    aiMocks.saveConfig.mockReset().mockImplementation(async (config) => ({ ...engine, config }));
    updaterMocks.configuration.mockReset().mockResolvedValue({
      configured: false,
      endpoint: "https://github.com/BayHen/app-change-voice/releases/latest/download/latest.json",
      signatureVerificationRequired: true,
      allowDowngrades: false,
    });
    updaterMocks.check.mockReset().mockResolvedValue(null);
    updaterMocks.relaunch.mockReset().mockResolvedValue(undefined);
    aiMocks.models.mockReset().mockResolvedValue([]);
    aiMocks.status.mockReset().mockResolvedValue({
      offlineOnly: true,
      inferenceBackend: "Python/OpenVoice prototype; native runtime deferred",
      gpuVerified: false,
      activeModelId: null,
      fallbackActive: true,
      message: "DSP fallback is active",
    });
    diagnosticsMocks.get.mockReset().mockResolvedValue({
      reportSchemaVersion: 1,
      generatedAtUnixSeconds: 0,
      applicationVersion: "0.1.0",
      platform: "windows-x86_64",
      activeInputDevice: null,
      activeOutputDevice: null,
      runtime: stoppedRuntime,
      routing: {
        state: "notConfigured",
        outputDeviceId: null,
        outputDeviceName: null,
        sampleRate: null,
        channels: null,
        underrunCount: 0,
        overflowCount: 0,
        outputPeakDbfs: -96,
        endpointVerified: false,
        message: null,
      },
      processingCpuEstimatePercent: null,
      privacyNotice: "Metadata only",
    });
    diagnosticsMocks.export.mockReset().mockResolvedValue(undefined);
    soundboardMocks.list.mockReset().mockResolvedValue([]);
    soundboardMocks.plugins.mockReset().mockResolvedValue([]);
    soundboardMocks.effects.mockReset().mockResolvedValue([]);
    const profiles = [
      {
        schemaVersion: 1,
        id: "valorant",
        name: "VALORANT Voice Chat",
        application: "valorant",
        presetId: "natural",
        noiseSuppression: true,
        limiterEnabled: true,
        soundboardEnabled: false,
        soundboardVolumePercent: 25,
        clarityPriority: true,
      },
    ];
    profileMocks.list.mockReset().mockResolvedValue(profiles);
    profileMocks.save.mockReset().mockImplementation(async (value) => value);
    profileMocks.remove.mockReset().mockResolvedValue(undefined);
    profileMocks.apply.mockReset().mockImplementation(async () => profiles[0]);
    backupMocks.status.mockReset().mockResolvedValue({
      provider: "local",
      enabled: false,
      state: "deferred",
      lastSyncedAt: null,
      message: "Cloud sync is deferred.",
    });
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
    effectMocks.get.mockReset().mockResolvedValue(effectParameters);
    effectMocks.set.mockReset().mockImplementation(async (parameters) => parameters);
    effectMocks.reset.mockReset().mockResolvedValue(effectParameters);
    presetMocks.list.mockReset().mockResolvedValue(builtInPresets);
    presetMocks.create
      .mockReset()
      .mockImplementation(async (preset) => ({ ...preset, id: "my-studio-voice" }));
    presetMocks.active.mockReset().mockResolvedValue("natural");
    presetMocks.apply.mockReset().mockImplementation(async (id) => ({
      preset: builtInPresets.find((item) => item.id === id),
      parameters: effectParameters,
    }));
    presetMocks.reset.mockReset().mockResolvedValue({
      preset: builtInPresets[0],
      parameters: effectParameters,
    });
    const routingStatus = {
      state: "ready",
      outputDeviceId: "wasapi:cable",
      outputDeviceName: null,
      sampleRate: null,
      channels: null,
      underrunCount: 0,
      overflowCount: 0,
      outputPeakDbfs: -96,
      endpointVerified: false,
      message: null,
    };
    routingMocks.devices.mockReset().mockResolvedValue([
      {
        ...connectedSnapshot.outputDevices[0],
        id: "wasapi:cable",
        name: "CABLE Input (VB-Audio Virtual Cable)",
        likelyVirtual: true,
      },
    ]);
    routingMocks.configuration
      .mockReset()
      .mockResolvedValue({ outputDeviceId: "wasapi:cable", outputDeviceFingerprint: "cable" });
    routingMocks.status.mockReset().mockResolvedValue(routingStatus);
    routingMocks.select
      .mockReset()
      .mockResolvedValue({ outputDeviceId: "wasapi:cable", outputDeviceFingerprint: "cable" });
    routingMocks.start
      .mockReset()
      .mockResolvedValue({ ...routingStatus, state: "active", endpointVerified: true });
    routingMocks.stop.mockReset().mockResolvedValue(routingStatus);
    routingMocks.openSettings.mockReset().mockResolvedValue(undefined);
    const hotkeyConfiguration = {
      schemaVersion: 1,
      bindings: [
        ["toggleMicrophoneMute", "Ctrl+Alt+M"],
        ["toggleVoiceEffects", "Ctrl+Alt+E"],
        ["nextVoicePreset", "Ctrl+Alt+Right"],
        ["previousVoicePreset", "Ctrl+Alt+Left"],
        ["toggleHearMyself", "Ctrl+Alt+H"],
        ["toggleVirtualMicrophone", "Ctrl+Alt+V"],
        ["toggleRecording", "Ctrl+Alt+R"],
      ].map(([action, shortcut]) => ({ action, shortcut })),
      tray: { minimizeToTray: false, closeToTray: true },
    };
    hotkeyMocks.get.mockReset().mockResolvedValue(hotkeyConfiguration);
    hotkeyMocks.set.mockReset().mockImplementation(async (value) => value);
    hotkeyMocks.reset.mockReset().mockResolvedValue(hotkeyConfiguration);
    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 900 });
    window.dispatchEvent(new Event("resize"));
  });

  it("browses female voices, persists favorites and never starts audio on browsing", async () => {
    localStorage.clear();
    presetMocks.list.mockResolvedValue([...builtInPresets, ...femalePresets]);
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Female collection" }));
    expect(await screen.findByRole("heading", { name: "21 voices" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Play voice preview" })).toBeDisabled();
    await user.type(screen.getByLabelText("Search female voices"), "Vietnamese Female");
    const favorite = screen.getByRole("button", { name: "Favorite Vietnamese Female" });
    await user.click(favorite);
    expect(favorite).toHaveAttribute("aria-pressed", "true");
    await user.selectOptions(screen.getByLabelText("Library scope"), "Favorites");
    expect(screen.getByRole("heading", { name: "1 voices" })).toBeVisible();
    expect(runtimeMocks.start).not.toHaveBeenCalled();
    expect(aiMocks.startConversion).not.toHaveBeenCalled();
    await user.click(screen.getAllByRole("button", { name: "Voice Lab" })[1]);
    await user.click(await screen.findByRole("button", { name: "Female collection" }));
    expect(
      await screen.findByRole("button", { name: "Favorite Vietnamese Female" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("applies the selected female DSP and keeps regional claims unverified", async () => {
    localStorage.clear();
    presetMocks.list.mockResolvedValue(femalePresets);
    const voice = femalePresets.find((v) => v.name === "Soft Female")!;
    const parameters = {
      ...(await effectMocks.get()),
      pitchSemitones: voice.pitchSemitones,
      formantSemitones: voice.formantSemitones,
    };
    presetMocks.apply.mockImplementation(async () => {
      effectMocks.get.mockResolvedValue(parameters);
      return { preset: voice, parameters };
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Female collection" }));
    await user.click(await screen.findByRole("button", { name: "Select Soft Female" }));
    await user.click(screen.getByRole("button", { name: "Use DSP voice" }));
    expect(presetMocks.apply).toHaveBeenCalledWith(voice.id);
    await waitFor(() => expect(screen.getByLabelText("Female pitch")).toHaveValue("1.4"));
    fireEvent.change(screen.getByLabelText("Female pitch"), { target: { value: "2" } });
    await waitFor(() =>
      expect(effectMocks.set).toHaveBeenCalledWith(expect.objectContaining({ pitchSemitones: 2 })),
    );
    await waitFor(() => expect(screen.getByLabelText("Female formant")).not.toBeDisabled());
    fireEvent.change(screen.getByLabelText("Female formant"), { target: { value: "1.5" } });
    expect(effectMocks.set).toHaveBeenCalledWith(
      expect.objectContaining({
        pitchSemitones: 2,
        formantSemitones: 1.5,
      }),
    );
    expect(presetMocks.apply).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/Vietnamese intelligibility is not yet verified/)).toBeVisible();
    expect(runtimeMocks.start).not.toHaveBeenCalled();
  });

  it("does not play a delayed file preview after leaving the collection", async () => {
    localStorage.clear();
    presetMocks.list.mockResolvedValue(femalePresets);
    previewMocks.open.mockResolvedValue("D:/local/consented.wav");
    let complete!: (value: unknown) => void;
    previewMocks.render.mockImplementation(
      () =>
        new Promise((resolve) => {
          complete = resolve;
        }),
    );
    const play = vi.fn().mockResolvedValue(undefined),
      AudioConstructor = vi.fn(function () {
        return { play, pause: vi.fn(), onended: null };
      });
    const create = URL.createObjectURL,
      revoke = URL.revokeObjectURL;
    URL.createObjectURL = vi.fn(() => "blob:test");
    URL.revokeObjectURL = vi.fn();
    vi.stubGlobal("Audio", AudioConstructor);
    try {
      const user = userEvent.setup();
      render(<App />);
      await user.click(await screen.findByRole("button", { name: "Female collection" }));
      await user.click(await screen.findByRole("button", { name: "Choose preview WAV" }));
      await screen.findByText("D:/local/consented.wav");
      await user.click(screen.getByLabelText("I have permission to use this sample."));
      await waitFor(() =>
        expect(screen.getByRole("button", { name: "Play voice preview" })).toBeEnabled(),
      );
      await user.click(screen.getByRole("button", { name: "Play voice preview" }));
      expect(previewMocks.render).toHaveBeenCalled();
      await user.click(screen.getAllByRole("button", { name: "Voice Lab" })[1]);
      await act(async () =>
        complete({
          wav: [82, 73, 70, 70],
          peak: 0.1,
          rms: 0.05,
          frames: 1,
          sampleRate: 48000,
          fullScaleSamples: 0,
          processingMs: 1,
        }),
      );
      expect(AudioConstructor).not.toHaveBeenCalled();
      expect(play).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllGlobals();
      URL.createObjectURL = create;
      URL.revokeObjectURL = revoke;
    }
  });

  it("blocks sound playback until the user explicitly starts the audio engine", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    soundboardMocks.list.mockResolvedValue([
      {
        id: "whoosh",
        name: "Whoosh",
        filePath: "D:/whoosh.wav",
        category: "General",
        favorite: false,
        volumePercent: 80,
        route: "headphones",
        hotkey: null,
        durationSeconds: 2.3,
        missing: false,
      },
    ]);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Soundboard" }));
    expect(await screen.findByRole("button", { name: "Play" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Stop" })).toBeDisabled();
    expect(runtimeMocks.start).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Start audio engine" }));
    expect(runtimeMocks.start).toHaveBeenCalledWith("wasapi:microphone-1", "wasapi:speakers-1");
  });

  it("navigates all five workspaces and preserves access to legacy capabilities", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(screen.getByRole("heading", { name: "Voice Lab" })).toBeInTheDocument();
    for (const label of ["Soundboard", "AI Voice Studio", "Recordings", "Settings"]) {
      await user.click(screen.getByRole("button", { name: label }));
      expect(screen.getByRole("heading", { name: label })).toBeInTheDocument();
      expect(screen.getByRole("contentinfo", { name: "Audio status" })).toBeInTheDocument();
    }
    expect(screen.getByRole("button", { name: /Backup Now/ })).toBeInTheDocument();
    expect(screen.getByText("Audio Diagnostics")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Check for updates/ })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Games & apps" }));
    expect(screen.getByRole("heading", { name: "Game & App Integration" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Virtual microphone" }));
    expect(await screen.findByLabelText("Virtual cable playback endpoint")).toHaveValue(
      "wasapi:cable",
    );
  });

  it("fills the recording timeline at the exact fractional end time", async () => {
    recordingMocks.status.mockResolvedValue({
      recording: false,
      playing: false,
      hearMyself: false,
      source: "processed",
      durationSeconds: 5.8,
      playbackSeconds: 5.8,
      droppedFrames: 0,
      hasRecording: true,
      incomplete: false,
      error: null,
    });
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Recordings" }));
    const timeline = (await screen.findByRole("progressbar", {
      name: "Playback timeline",
    })) as HTMLProgressElement;
    await waitFor(() => expect(timeline.position).toBe(1));
    expect(timeline).toHaveAttribute("aria-valuetext", "00:05.8 of 00:05.8");
  });

  it("renders disconnected microphone state without fake live data", async () => {
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    await userEvent.click(screen.getByRole("button", { name: "Audio devices" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Microphone is not connected");
    expect(screen.getByLabelText("Input device")).toBeDisabled();
    expect(screen.getByRole("meter", { name: "Input level" })).toHaveAttribute(
      "aria-valuenow",
      "-60",
    );
    expect(screen.getByRole("button", { name: "Start recording" })).toBeDisabled();
  });

  it("records, detects duplicates, removes and saves global shortcuts", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getAllByRole("button", { name: "Settings" })[0]);
    const assign = await screen.findByRole("button", {
      name: "Assign shortcut for Toggle microphone mute",
    });
    await user.click(assign);
    fireEvent.keyDown(assign, { key: "E", ctrlKey: true, altKey: true });
    expect(
      screen.getByText("Duplicate shortcuts must be resolved before saving."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save hotkeys" })).toBeDisabled();
    await user.click(screen.getAllByRole("button", { name: "Remove" })[0]);
    expect(screen.getByRole("button", { name: "Save hotkeys" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Save hotkeys" }));
    expect(hotkeyMocks.set).toHaveBeenCalled();
  });

  it("keeps monitoring and recording alive while navigating all workspaces", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const user = userEvent.setup();
    render(<App />);
    const monitor = await screen.findByRole("switch", { name: "Monitor" });
    await user.click(monitor);
    expect(runtimeMocks.start).toHaveBeenCalledOnce();
    expect(await screen.findByText("Audio engine running")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Record" }));
    expect(recordingMocks.start).toHaveBeenCalledOnce();
    for (const name of ["Soundboard", "AI Voice Studio", "Recordings", "Settings", "Voice Lab"]) {
      await user.click(screen.getByRole("button", { name }));
      expect(screen.getByRole("switch", { name: "Monitor" })).toHaveAttribute(
        "aria-checked",
        "true",
      );
    }
    expect(runtimeMocks.start).toHaveBeenCalledOnce();
    expect(runtimeMocks.stop).not.toHaveBeenCalled();
    expect(recordingMocks.stop).not.toHaveBeenCalled();
  });

  it("opens the preset dialog with focus and closes it with Escape", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("option", { name: "Studio Microphone" });
    const trigger = screen.getByRole("button", { name: "New preset" });
    await user.click(trigger);
    expect(screen.getByRole("dialog", { name: "Save your voice" })).toBeInTheDocument();
    expect(screen.getByLabelText("Preset name")).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("rejects a saved headphone route and shares cable selection with the persistent audio session", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const cable = (await routingMocks.devices())[0];
    const headphones = { ...connectedSnapshot.outputDevices[0], likelyVirtual: false };
    routingMocks.devices.mockResolvedValue([cable, headphones]);
    routingMocks.configuration.mockResolvedValue({
      outputDeviceId: headphones.id,
      outputDeviceFingerprint: "headphones",
    });
    await runtimeMocks.start();
    runtimeMocks.start.mockClear();
    routingMocks.select.mockImplementation(async (id) => {
      currentRuntime = stoppedRuntime;
      return { outputDeviceId: id, outputDeviceFingerprint: "cable" };
    });
    const ready = await routingMocks.status();
    routingMocks.start.mockImplementation(async () => {
      const active = { ...ready, state: "active", endpointVerified: true };
      routingMocks.status.mockResolvedValue(active);
      return active;
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: /^Settings$/ }));
    await user.click(screen.getByRole("button", { name: "Virtual microphone" }));
    await screen.findByText(/The saved route points to headphones/);
    expect(screen.getByRole("button", { name: "Start routing" })).toBeDisabled();
    expect(screen.getByRole("option", { name: /^Desktop Speakers/ })).toBeDisabled();
    await user.selectOptions(screen.getByLabelText("Virtual cable playback endpoint"), cable.id);
    await screen.findByText("Audio engine idle");
    expect(screen.getByRole("button", { name: "Start routing" })).toBeDisabled();
    await user.click(screen.getByRole("switch", { name: "Monitor" }));
    await screen.findByText("Audio engine running");
    await user.click(screen.getByRole("button", { name: "Start routing" }));
    await screen.findByText("Routing active");
    expect(routingMocks.select).toHaveBeenCalledWith(cable.id);
    expect(routingMocks.start).toHaveBeenCalledOnce();
  });

  it("requires explicit local WAV consent and a running session in Audio Quality Lab", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: /^Settings$/ }));
    await user.click(screen.getByRole("button", { name: "Audio Quality Lab" }));
    const record = await screen.findByRole("button", { name: "Record selected diagnostic stages" });
    expect(record).toBeDisabled();
    expect(qualityMocks.start).not.toHaveBeenCalled();
    expect(runtimeMocks.start).not.toHaveBeenCalled();
    await user.click(screen.getByRole("switch", { name: "Monitor" }));
    await screen.findByText("Audio engine running");
    expect(record).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: /I explicitly allow recording/ }));
    await user.click(record);
    expect(qualityMocks.start).toHaveBeenCalledWith(["mixer", "virtualOutput"], 5, true);
    expect(screen.getByRole("button", { name: "Stop diagnostic capture" })).toBeEnabled();
    await user.selectOptions(screen.getByLabelText("Audio quality profile"), "highQuality");
    expect(screen.getByRole("button", { name: "Apply audio quality" })).toBeDisabled();
  });

  it("shows a stopped engine after fatal driver failure and clears the alert on explicit retry", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const running = await runtimeMocks.start();
    currentRuntime = {
      ...running,
      state: "faulted",
      metrics: {
        ...running.metrics,
        streamFaulted: true,
        streamErrorCount: 1,
        lastStreamError: "Audio output: device disconnected",
      },
    };
    runtimeMocks.start.mockClear();
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Audio engine stopped after error");
    expect(screen.getByRole("switch", { name: "Monitor" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    expect(screen.getByRole("button", { name: /^Record$/ })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("Audio output: device disconnected");
    expect(runtimeMocks.start).not.toHaveBeenCalled();
    await user.click(screen.getByRole("switch", { name: "Monitor" }));
    await screen.findByText("Audio engine running");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("saves edited DSP settings and enables export for the newly created preset", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    volumeMocks.get.mockResolvedValue({
      inputPercent: 65,
      outputPercent: 75,
      inputMuted: false,
      outputMuted: false,
    });
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("option", { name: "Studio Microphone" });
    fireEvent.change(screen.getByLabelText("Pitch"), { target: { value: "2" } });
    fireEvent.change(screen.getByLabelText("Bass"), { target: { value: "3" } });
    await user.click(screen.getByRole("button", { name: "New preset" }));
    const saves = screen.getAllByRole("button", { name: "Save preset" });
    await user.click(saves[saves.length - 1]);
    expect(presetMocks.create).toHaveBeenCalledWith(
      expect.objectContaining({
        pitchSemitones: 2,
        parameters: expect.objectContaining({
          bassDb: 3,
          inputGainPercent: 65,
          outputGainPercent: 75,
        }),
      }),
    );
    expect(await screen.findByText("Preset saved to your local library.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Export" })).toBeEnabled();
  });

  it("filters and applies a validated backend preset", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("button", { name: "Voice presets" }));
    await user.type(screen.getByLabelText("Search presets"), "Deep");

    expect(await screen.findByRole("button", { name: /Deep Voice/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Natural Voice/i })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Deep Voice/i }));
    await user.click(screen.getByRole("button", { name: "Apply preset" }));
    expect(presetMocks.apply).toHaveBeenCalledWith("deep");
    expect(
      await screen.findByText("Preset validated and applied to the realtime Rust DSP chain."),
    ).toBeInTheDocument();
  });

  it("renders backend devices and refreshes without opening an audio stream", async () => {
    audioMocks.refresh.mockResolvedValue(connectedSnapshot);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("button", { name: "Audio devices" }));

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
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("button", { name: "Audio devices" }));

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
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("button", { name: "Audio devices" }));

    const inputGain = await screen.findByLabelText("Microphone input gain");
    fireEvent.change(inputGain, { target: { value: "65" } });
    expect(volumeMocks.preview).toHaveBeenCalledWith(expect.objectContaining({ inputPercent: 65 }));

    await user.click(screen.getByRole("button", { name: "Mute microphone" }));
    expect(volumeMocks.set).toHaveBeenCalledWith(expect.objectContaining({ inputMuted: true }));
    expect(screen.getByLabelText("Output monitoring volume")).toHaveValue("100");
  });
});

describe("signed updater UI", () => {
  it("reports a network check failure without crashing the application", async () => {
    updaterMocks.configuration.mockResolvedValue({
      configured: true,
      endpoint: "https://github.com/BayHen/app-change-voice/releases/latest/download/latest.json",
      signatureVerificationRequired: true,
      allowDowngrades: false,
    });
    updaterMocks.check.mockRejectedValue(new Error("network unavailable"));
    render(<UpdateSettings />);
    const checkButton = await screen.findByRole("button", { name: /Check for updates/ });
    await userEvent.click(checkButton);
    expect(await screen.findByRole("alert")).toHaveTextContent(/network unavailable/);
  });
});

describe("voice controls", () => {
  it("sends pitch, formant, bypass and reset controls to the DSP backend", async () => {
    const effectParameters = {
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
