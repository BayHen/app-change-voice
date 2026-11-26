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

export type MonitoringState = "stopped" | "running" | "paused" | "faulted";

export interface AudioMetrics {
  sampleRate: number;
  inputBufferFrames: number;
  outputBufferFrames: number;
  dspLatencyFrames: number;
  underrunCount: number;
  overflowCount: number;
  streamErrorCount: number;
  lastStreamError?: string | null;
  averageProcessingMicros: number;
  estimatedLatencyMs: number;
  ringDepthFrames: number;
  inputLevel: number;
  outputLevel: number;
  inputPeakDbfs: number;
  inputRmsDbfs: number;
  outputPeakDbfs: number;
  outputRmsDbfs: number;
  inputClipping: boolean;
  outputClipping: boolean;
  streamFaulted: boolean;
  protectedClipCount: number;
  captureCallbackMaxMicros?: number;
  outputCallbackMaxMicros?: number;
  playbackAlgorithmFrames?: number;
  driftCorrectionPpm?: number;
  discardedFrames?: number;
}

export interface VolumeSettings {
  inputPercent: number;
  outputPercent: number;
  inputMuted: boolean;
  outputMuted: boolean;
}

export interface NoiseSettings {
  enabled: boolean;
  bypassed: boolean;
  suppressionStrength: number;
  gateEnabled: boolean;
  gateThresholdDbfs: number;
  limiterEnabled: boolean;
  limiterCeilingDbfs: number;
}

export type RecordingSource = "original" | "processed";
export interface RecordingStatus {
  recording: boolean;
  playing: boolean;
  hearMyself: boolean;
  source: RecordingSource;
  durationSeconds: number;
  playbackSeconds: number;
  droppedFrames: number;
  hasRecording: boolean;
  incomplete: boolean;
  error: string | null;
}

export interface EffectParameters {
  voiceIntensity?: number;
  deEssStrength?: number;
  compressionRatio?: number;
  pitchSemitones: number;
  formantSemitones: number;
  enabled: boolean;
  bypassed: boolean;
  makeupGainDb: number;
  ringModFrequencyHz: number;
  ringModMix: number;
  radioHighPassHz: number;
  radioLowPassHz: number;
  radioDrive: number;
  radioMix: number;
  tremoloRateHz: number;
  tremoloDepth: number;
  bassDb: number;
  trebleDb: number;
}

export interface AudioRuntimeStatus {
  state: MonitoringState;
  inputDeviceId: string | null;
  outputDeviceId: string | null;
  sampleRate: number | null;
  inputChannels: number | null;
  outputChannels: number | null;
  dspLatencyFrames: number | null;
  metrics: AudioMetrics | null;
  inputSampleFormat?: string | null;
  outputSampleFormat?: string | null;
  diagnosticRecording?: boolean;
}

export type AudioQualityProfile = "lowLatency" | "balanced" | "highQuality";
export interface AudioQualitySettings {
  profile: AudioQualityProfile;
}
export type AudioCaptureStage =
  | "rawInput"
  | "bypassReference"
  | "inputGain"
  | "classicDsp"
  | "aiInference"
  | "mixer"
  | "monitor"
  | "pcmBridge"
  | "virtualOutput";
export interface StageCaptureStatus {
  running: boolean;
  folder: string | null;
  droppedSamples: number;
  message: string | null;
  files: Array<{
    stage: AudioCaptureStage;
    sampleRate: number;
    channels: number;
    sampleFormat: string;
    frames: number;
    peak: number;
    rms: number;
    clippedSamples: number;
    nonFiniteSamples: number;
    file: string;
  }>;
}

export interface VirtualOutputDevice extends AudioDevice {
  likelyVirtual: boolean;
}
export interface RoutingConfiguration {
  outputDeviceId: string | null;
  outputDeviceFingerprint: string | null;
}
export type RoutingState = "notConfigured" | "ready" | "active" | "faulted";
export interface RoutingStatus {
  state: RoutingState;
  outputDeviceId: string | null;
  outputDeviceName: string | null;
  sampleRate: number | null;
  channels: number | null;
  underrunCount: number;
  overflowCount: number;
  outputPeakDbfs: number;
  endpointVerified: boolean;
  message: string | null;
}

export interface AudioDiagnosticsReport {
  reportSchemaVersion: number;
  generatedAtUnixSeconds: number;
  applicationVersion: string;
  platform: string;
  activeInputDevice: string | null;
  activeOutputDevice: string | null;
  runtime: AudioRuntimeStatus;
  routing: RoutingStatus;
  processingCpuEstimatePercent: number | null;
  privacyNotice: string;
}

export interface AudioUiState {
  inputDeviceId: string;
  outputDeviceId: string;
  microphoneVolume: number;
  outputVolume: number;
  noiseSuppression: boolean;
  monitoring: boolean;
}
