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

export interface AudioUiState {
  inputDeviceId: string;
  outputDeviceId: string;
  microphoneVolume: number;
  outputVolume: number;
  noiseSuppression: boolean;
  monitoring: boolean;
}
