export type DeviceKind = "input" | "output";

export interface DeviceOption {
  id: string;
  name: string;
  kind: DeviceKind;
  available: boolean;
  isDefault?: boolean;
}

export interface BufferCapability {
  minimumFrames: number | null;
  maximumFrames: number | null;
}

export interface AudioDevice {
  id: string;
  name: string;
  kind: DeviceKind;
  isDefault: boolean;
  sampleRate: number | null;
  channels: number | null;
  sampleFormat: string | null;
  bufferCapability: BufferCapability | null;
  fingerprint: string;
}

export interface DeviceSelection {
  selectedId: string | null;
  fellBack: boolean;
  message: string | null;
}

export interface AudioDeviceSnapshot {
  inputDevices: AudioDevice[];
  outputDevices: AudioDevice[];
  inputSelection: DeviceSelection;
  outputSelection: DeviceSelection;
}

export interface DefaultDevices {
  inputId: string | null;
  outputId: string | null;
}

export type MonitoringState = "stopped" | "running" | "paused";

export interface AudioMetrics {
  sampleRate: number;
  inputBufferFrames: number;
  outputBufferFrames: number;
  underrunCount: number;
  overflowCount: number;
  streamErrorCount: number;
  averageProcessingMicros: number;
  estimatedLatencyMs: number;
  ringDepthFrames: number;
  inputLevel: number;
  outputLevel: number;
  streamFaulted: boolean;
}

export interface AudioRuntimeStatus {
  state: MonitoringState;
  inputDeviceId: string | null;
  outputDeviceId: string | null;
  sampleRate: number | null;
  inputChannels: number | null;
  outputChannels: number | null;
  metrics: AudioMetrics | null;
}

export interface AudioUiState {
  inputDeviceId: string;
  outputDeviceId: string;
  microphoneVolume: number;
  outputVolume: number;
  noiseSuppression: boolean;
  monitoring: boolean;
}
