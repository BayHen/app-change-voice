export interface AIModelStatus {
  schemaVersion: number;
  id: string;
  name: string;
  description: string;
  architecture: string;
  backend: string;
  license: string;
  sourceUrl: string;
  modelFile: string;
  sha256: string;
  inputSampleRate: number;
  frameSamples: number;
  supportedProfiles: string[];
  enabled: boolean;
  integrityVerified: boolean;
  runtimeAvailable: boolean;
  statusMessage: string;
  consentConfirmed?: boolean;
  licenseUrl?: string | null;
  attribution?: string | null;
}

export interface AIEngineConfig {
  provider: "auto" | "cpu" | "cuda";
  allowCpuFallback: boolean;
  failurePolicy: "mute" | "classicDsp";
  pitchSemitones: number;
  f0Method: "dio" | "harvest" | "rmvpe_onnx";
  chunkFrames: number;
  crossfadeFrames: number;
  extraContextFrames: number;
  noiseThreshold: number;
  cpuThreads: number;
}
export interface AIEngineStatus {
  state: string;
  mode: "classicDsp" | "wOkada";
  classicFallbackActive: boolean;
  runtimeVersion: string | null;
  recommendedRuntimeVersion: string;
  recommendedDownloadBytes: number;
  download: {
    phase: string;
    component: string;
    completedBytes: number;
    totalBytes: number;
    completedComponents: number;
    totalComponents: number;
  } | null;
  hardware: {
    pythonVersion: string;
    torchVersion: string;
    onnxRuntimeVersion: string;
    providers: string[];
    cudaDevices: string[];
    gpuMemoryBytes: number[];
    cpuThreads: number;
    torchCudaAvailable: boolean;
    torchCudaVersion: string | null;
  } | null;
  runtimeInstalled: boolean;
  modelId: string | null;
  provider: string | null;
  upstreamCommit: string;
  loadingMs: number | null;
  conversionMs: number | null;
  roundTripMs: number | null;
  queuedInputSamples: number;
  queuedOutputSamples: number;
  droppedInputSamples: number;
  droppedOutputSamples: number;
  missingOutputSamples: number;
  completedChunks: number;
  restarts: number;
  message: string | null;
  config: AIEngineConfig;
  install: {
    installed: boolean;
    installing: boolean;
    copiedFiles: number;
    totalFiles: number;
    version: string | null;
    message: string | null;
  };
}

export interface AIProviderStatus {
  offlineOnly: boolean;
  inferenceBackend: string;
  gpuVerified: boolean;
  activeModelId: string | null;
  fallbackActive: boolean;
  message: string;
}
