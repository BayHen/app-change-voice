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
