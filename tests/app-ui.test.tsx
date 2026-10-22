import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../src/App";
import { VoiceStudio } from "../src/pages/VoiceStudio";

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

const stoppedRuntime = {
  state: "stopped",
  inputDeviceId: null,
  outputDeviceId: null,
  sampleRate: null,
  inputChannels: null,
  outputChannels: null,
  metrics: null,
};

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
    runtimeMocks.status.mockReset().mockResolvedValue(stoppedRuntime);
    runtimeMocks.stop.mockReset().mockResolvedValue(stoppedRuntime);
    runtimeMocks.pause.mockReset().mockResolvedValue({ ...stoppedRuntime, state: "paused" });
    runtimeMocks.resume.mockReset().mockResolvedValue({ ...stoppedRuntime, state: "running" });
    runtimeMocks.start.mockReset().mockResolvedValue({
      ...stoppedRuntime,
      state: "running",
      inputDeviceId: "wasapi:microphone-1",
      outputDeviceId: "wasapi:speakers-1",
      sampleRate: 48000,
      inputChannels: 2,
      outputChannels: 2,
      metrics: {
        sampleRate: 48000,
        inputBufferFrames: 256,
        outputBufferFrames: 256,
        underrunCount: 0,
        overflowCount: 0,
        streamErrorCount: 0,
        averageProcessingMicros: 0.1,
        estimatedLatencyMs: 10.7,
        ringDepthFrames: 128,
        inputLevel: 0.25,
        outputLevel: 0.25,
        streamFaulted: false,
      },
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
      "0",
    );
    expect(screen.getByRole("button", { name: /Record & Test/i })).toBeDisabled();
  });

  it("filters presets and reports that selection is UI-only", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getAllByRole("button", { name: "Voice Effects" })[0]);
    await user.type(screen.getByLabelText("Search presets"), "Deep");

    expect(screen.getByRole("button", { name: /Deep Character/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Clean Voice/i })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Deep Character/i }));
    expect(screen.getByRole("status")).toHaveTextContent("UI state only");
    expect(screen.getByRole("button", { name: /Apply preset/i })).toBeDisabled();
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
});

describe("voice controls", () => {
  it("updates local slider and bypass values without claiming backend processing", async () => {
    const user = userEvent.setup();
    render(<VoiceStudio />);

    const pitch = screen.getByLabelText("Pitch");
    fireEvent.change(pitch, { target: { value: "4" } });
    expect(pitch).toHaveValue("4");
    expect(screen.getAllByText("4 st")).toHaveLength(2);

    const bypass = screen.getByRole("switch", { name: "Effect bypass" });
    expect(bypass).toHaveAttribute("aria-checked", "false");
    await user.click(bypass);
    expect(bypass).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("button", { name: /Save preset/i })).toBeDisabled();
  });
});
