import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { AgentVoiceCollection } from "../src/components/AgentVoiceCollection";
import { NeuralModelImport } from "../src/components/NeuralModelImport";
import { VoicePackManager } from "../src/components/VoicePackManager";
const m = vi.hoisted(() => ({
  voices: vi.fn(),
  status: vi.fn(),
  models: vi.fn(),
  assign: vi.fn(),
  remove: vi.fn(),
  start: vi.fn(),
  save: vi.fn(),
  classic: vi.fn(),
  refresh: vi.fn(),
  stop: vi.fn(),
  compare: vi.fn(),
  calibrate: vi.fn(),
  effects: vi.fn(),
  open: vi.fn(),
  inspect: vi.fn(),
  register: vi.fn(),
  legacy: vi.fn(),
  packStatus: vi.fn(),
  installPack: vi.fn(),
  inspectHF: vi.fn(),
}));
const effects = {
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
vi.mock("../src/hooks/AudioSession", () => ({
  useAudioSession: () => ({
    ai: {
      status: {
        state: "stopped",
        runtimeInstalled: true,
        config: { provider: "cuda", hybridDsp: false, failurePolicy: "mute" },
        ...m.status(),
      },
      save: m.save,
      classic: m.classic,
      refresh: m.refresh,
      stop: m.stop,
    },
    routing: { status: { state: "notConfigured" } },
  }),
}));
vi.mock("../src/hooks/useEffectParameters", () => ({
  useEffectParameters: () => ({ parameters: effects, update: m.effects }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: m.open }));
vi.mock("../src/services/tauri", () => ({
  getAgentVoices: m.voices,
  getAIModels: m.models,
  assignAgentVoice: m.assign,
  removeAgentVoice: m.remove,
  startAgentVoice: m.start,
  compareAgentVoice: m.compare,
  calibrateAgentVoice: m.calibrate,
  inspectVoiceModel: m.inspect,
  registerRawVoiceModel: m.register,
  addAIModel: m.legacy,
  getVoicePackStatus: m.packStatus,
  installVoicePack: m.installPack,
  inspectHFVoice: m.inspectHF,
}));
const voices = [
  { id: "wind-assassin", name: "Wind Assassin" },
  { id: "jade-guardian", name: "Jade Guardian" },
  { id: "jett", name: "Jett-inspired" },
].map((voice) => ({
  ...voice,
  state: "notInstalled",
  binding: null,
  message: "No independent neural identity installed.",
}));
const model = {
  id: "owner-voice",
  name: "Authorized original identity",
  backend: "w-okada-rvc",
  integrityVerified: true,
  consentConfirmed: true,
  sha256: "a".repeat(64),
};
const binding = {
  modelId: model.id,
  sha256: model.sha256,
  identityName: "Original owner",
  rightsConfirmed: true,
  qualityMode: "balanced",
};
beforeEach(() => {
  vi.clearAllMocks();
  m.voices.mockResolvedValue(voices);
  m.status.mockReturnValue({});
  m.packStatus.mockResolvedValue({ phase: "idle", completedBytes: 0, totalBytes: 0 });
  m.models.mockResolvedValue([model]);
  m.assign.mockResolvedValue(undefined);
  m.save.mockResolvedValue({});
  m.refresh.mockResolvedValue(undefined);
});
it("shows the engine's live failure and leaves a validated voice available to retry", async () => {
  m.voices.mockResolvedValue([{ ...voices[0], state: "ready", binding }]);
  m.status.mockReturnValue({
    state: "faulted",
    modelId: model.id,
    message: "AI operation timed out or the sidecar crashed",
  });
  render(<AgentVoiceCollection onBack={vi.fn()} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("AI operation timed out");
  await waitFor(() => expect(screen.getByRole("button", { name: "Load neural voice" })).toBeEnabled());
});
it("shows only the two original profiles and keeps both inactive without weights", async () => {
  render(<AgentVoiceCollection onBack={vi.fn()} />);
  expect(await screen.findAllByText("Missing Model")).toHaveLength(2);
  expect(screen.getByText("Wind Assassin", { selector: "strong" })).toBeVisible();
  expect(screen.getByText("Jade Guardian", { selector: "strong" })).toBeVisible();
  expect(screen.queryByText("Jett-inspired")).toBeNull();
  expect(screen.getByRole("button", { name: "Load neural voice" })).toBeDisabled();
  expect(m.start).not.toHaveBeenCalled();
});

it("uses the original pack manifest picker and requires consent before installing", async () => {
  const user = userEvent.setup();
  const path = "D:\\Voice packs\\voice-pack.json";
  m.open.mockResolvedValueOnce(path);
  m.installPack.mockResolvedValueOnce({ phase: "preparing", agentId: "jade-guardian" });
  render(<AgentVoiceCollection onBack={vi.fn()} />);
  await user.click(await screen.findByRole("button", { name: "Import model for Jade Guardian" }));
  await user.click(screen.getByRole("button", { name: "Choose voice-pack.json" }));
  expect(m.open).toHaveBeenCalledWith(
    expect.objectContaining({
      filters: [{ name: "Original RVC voice pack", extensions: ["json"] }],
    }),
  );
  const install = screen.getByRole("button", { name: "Install, validate & assign" });
  expect(install).toBeDisabled();
  await user.click(screen.getByRole("checkbox", { name: /I have permission to use this model/ }));
  await user.click(install);
  expect(m.installPack).toHaveBeenCalledWith(
    expect.objectContaining({
      agentId: "jade-guardian",
      localPath: path,
      hfUrl: null,
      rightsConfirmed: true,
    }),
  );
});
it("loads validated voices through neural IPC and connects optional post effects", async () => {
  const u = userEvent.setup();
  m.voices.mockResolvedValue([{ ...voices[0], state: "ready", binding }]);
  render(<AgentVoiceCollection onBack={vi.fn()} />);
  await screen.findByText("Wind Assassin", { selector: "strong" });
  await u.selectOptions(screen.getByLabelText("Conversion engine"), "hybrid");
  fireEvent.change(screen.getByRole("slider", { name: "Radio effect" }), {
    target: { value: "0.25" },
  });
  expect(m.effects).toHaveBeenCalledWith(
    expect.objectContaining({ radioMix: 0.25, pitchSemitones: 0 }),
  );
  await u.click(screen.getByRole("button", { name: "Load neural voice" }));
  await waitFor(() => expect(m.start).toHaveBeenCalledWith("wind-assassin"));
  expect(m.save).toHaveBeenCalledWith(
    expect.objectContaining({ hybridDsp: true, failurePolicy: "mute" }),
  );
});
it("shows the pending voice beside its activation controls", async () => {
  m.voices.mockResolvedValue([{ ...voices[0], state: "ready", binding }]);
  let finish!: () => void;
  m.start.mockImplementationOnce(() => new Promise<void>((resolve) => { finish = resolve; }));
  render(<AgentVoiceCollection onBack={vi.fn()} />);
  await userEvent.click(await screen.findByRole("button", { name: "Activate Voice" }));
  expect(screen.getByRole("progressbar", { name: "Neural voice loading" })).toBeVisible();
  expect(screen.queryByRole("progressbar", { name: "Agent voice operation progress" })).toBeNull();
  finish();
  await waitFor(() => expect(screen.queryByRole("progressbar", { name: "Neural voice loading" })).toBeNull());
});
it("requires recording consent before local calibration or A/B processing", async () => {
  const u = userEvent.setup();
  m.voices.mockResolvedValue([{ ...voices[0], state: "imported", binding }]);
  m.open.mockResolvedValueOnce("C:\\My recordings\\giọng.wav");
  m.calibrate.mockResolvedValueOnce({
    estimatedF0Hz: null,
    f0SpreadHz: null,
    rmsDbfs: -30,
    peakDbfs: -8,
    clippingRate: 0,
    quietFrameDbfs: -50,
    recommendedGainDb: 6,
  });
  render(<AgentVoiceCollection onBack={vi.fn()} />);
  await screen.findByText("Wind Assassin", { selector: "strong" });
  await u.click(screen.getByRole("button", { name: "Choose local WAV" }));
  expect(screen.getByRole("button", { name: "Render four-way comparison" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Calibrate input for this voice" })).toBeDisabled();
  await u.click(
    screen.getByRole("checkbox", { name: /I have permission to process this recording/ }),
  );
  await u.click(screen.getByRole("button", { name: "Calibrate input for this voice" }));
  await waitFor(() =>
    expect(m.calibrate).toHaveBeenCalledWith("wind-assassin", "C:\\My recordings\\giọng.wav", true),
  );
  expect(m.compare).not.toHaveBeenCalled();
  expect(await screen.findByText(/Unvoiced \/ unavailable/)).toBeInTheDocument();
});
it("offers raw ONNX and JSON filters and blocks incompatible model registration", async () => {
  const u = userEvent.setup();
  m.open.mockResolvedValueOnce("C:\\voices\\sample.onnx");
  m.inspect.mockResolvedValueOnce({
    compatibility: "incompatible",
    issues: ["Unsupported tensor contract"],
    sha256: model.sha256,
    metadata: {},
    inputs: [],
    outputs: [],
  });
  render(<NeuralModelImport onClose={vi.fn()} onImported={vi.fn()} />);
  await u.click(screen.getByRole("button", { name: "Choose ONNX or JSON" }));
  expect(m.open).toHaveBeenCalledWith(
    expect.objectContaining({
      filters: expect.arrayContaining([expect.objectContaining({ extensions: ["onnx", "json"] })]),
    }),
  );
  expect(await screen.findByText("Unsupported tensor contract")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Register neural model" })).toBeDisabled();
  expect(m.register).not.toHaveBeenCalled();
});

it("keeps imported but unvalidated voices inactive", async () => {
  m.voices.mockResolvedValue([{ ...voices[0], state: "imported", binding }]);
  render(<AgentVoiceCollection onBack={vi.fn()} />);
  await screen.findByText("Wind Assassin", { selector: "strong" });
  expect(screen.getByRole("button", { name: "Load neural voice" })).toBeDisabled();
  expect(m.start).not.toHaveBeenCalled();
});

it("keeps the remote candidate blocked when the repository has no license", async () => {
  const u = userEvent.setup();
  m.inspectHF.mockResolvedValue({
    state: "licenseUnverified",
    declaredLicense: null,
    filename: "jett/jett.pth",
    bytes: 55223738,
    sha256: "a".repeat(64),
    revision: "b".repeat(40),
  });
  render(
    <VoicePackManager
      agentId="jett"
      models={[]}
      preferRemote
      onClose={vi.fn()}
      onChanged={vi.fn()}
      onActivate={vi.fn()}
    />,
  );
  await u.click(screen.getByRole("button", { name: "Check repository metadata" }));
  expect(await screen.findByText("License Unverified")).toBeVisible();
  expect(screen.getByRole("button", { name: "Install, validate & assign" })).toBeDisabled();
  expect(m.installPack).not.toHaveBeenCalled();
});

it("passes a local Unicode checkpoint path and explicit rights to the pack installer", async () => {
  const u = userEvent.setup();
  const path = "C:\\My voices\\giọng.pth";
  m.open.mockResolvedValueOnce(path);
  m.installPack.mockResolvedValue({ phase: "preparing", agentId: "sage" });
  render(
    <VoicePackManager
      agentId="sage"
      models={[]}
      onClose={vi.fn()}
      onChanged={vi.fn()}
      onActivate={vi.fn()}
    />,
  );
  await u.click(screen.getByRole("button", { name: "Choose PTH or ONNX" }));
  expect(m.open).toHaveBeenCalledWith(
    expect.objectContaining({
      filters: [{ name: "RVC model", extensions: ["pth", "onnx"] }],
    }),
  );
  const install = screen.getByRole("button", { name: "Install, validate & assign" });
  expect(install).toBeDisabled();
  await u.click(screen.getByRole("checkbox", { name: /I have model and voice authorization/ }));
  await u.click(install);
  expect(m.installPack).toHaveBeenCalledWith(
    expect.objectContaining({
      agentId: "sage",
      localPath: path,
      hfUrl: null,
      rightsConfirmed: true,
    }),
  );
  expect(screen.queryByRole("button", { name: "Activate validated voice" })).toBeNull();
});
