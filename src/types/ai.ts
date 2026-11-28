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
  hybridDsp?: boolean;
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
  supportedF0Methods?: string[];
  resources?: NeuralResources | null;
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
  missedDeadlines?: number;
  startupWaitSamples?: number;
  inferenceBudgetMs?: number | null;
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

export interface AgentBinding {
  weightsSha256?: string | null;
  modelId: string;
  sha256: string;
  identityName: string;
  rightsConfirmed: boolean;
  qualityMode: "lowLatency" | "balanced" | "highQuality";
}
export interface AgentVoice {
  id: string;
  name: string;
  state: "notInstalled" | "imported" | "ready" | "error";
  binding: AgentBinding | null;
  message: string;
}
export interface VoiceAnalysis {
  durationSeconds: number;
  rmsDbfs: number;
  peakDbfs: number;
  clippingRate: number;
  estimatedF0Hz: number | null;
  f0SpreadHz: number | null;
  voicedFrames: number;
  quietFrameDbfs: number;
  recommendedGainDb: number;
}
export interface AgentComparison {
  warmupMs: number;
  resources: NeuralResources | null;
  original: import("../services/tauri").VoicePreview;
  dsp: import("../services/tauri").VoicePreview;
  neural: import("../services/tauri").VoicePreview;
  hybrid: import("../services/tauri").VoicePreview;
  inputAnalysis: VoiceAnalysis;
  neuralAnalysis: VoiceAnalysis;
  initializationMs: number;
  inferenceMs: number;
  realTimeFactor: number;
  worstChunkMs: number;
  deadlineMs: number;
  missedDeadlines: number;
  provider: string;
  inputSha256: string;
  modelSha256: string;
  sourceFrames: number;
}

export interface NeuralResources {
  rssBytes: number;
  cpuSeconds: number;
  cpuCorePercent: number | null;
  gpuUtilizationPercent: number | null;
  cudaAllocatedBytes: number | null;
  cudaReservedBytes: number | null;
  cudaPeakAllocatedBytes: number | null;
}
export interface VoicePackRemote {
  repository: string;
  revision: string;
  filename: string;
  bytes: number;
  sha256: string;
  declaredLicense: string | null;
  state: string;
  downloadUrl: string;
}
export interface VoicePackReceipt {
  modelSha256: string;
  runtimeDigest: string;
  provider: string;
  peak: number;
  chunks: number;
  inferenceMs: number;
  realTimeFactor: number;
  config: AIEngineConfig;
  sourceSha256: string;
  detected: Record<string, unknown>;
}
export interface VoicePackStatus {
  phase: string;
  agentId: string | null;
  modelId: string | null;
  message: string;
  completedBytes: number;
  totalBytes: number;
  detected: Record<string, unknown> | null;
  validation: VoicePackReceipt | null;
}
export interface VoicePackInstallRequest {
  agentId: string;
  localPath: string | null;
  hfUrl: string | null;
  existingModelId: string | null;
  indexPath: string | null;
  identityName: string | null;
  sourceUrl: string | null;
  licenseUrl: string | null;
  attribution: string | null;
  rightsConfirmed: boolean;
}

export const runningPackPhases = new Set([
  "preparing",
  "checkingLicense",
  "checkingRuntime",
  "downloading",
  "converting",
  "installingConverter",
  "validating",
  "testingInference",
  "registering",
  "assigning",
]);
export const packPhaseLabels: Record<string, string> = {
  preparing: "Preparing voice pack",
  checkingLicense: "Checking repository license",
  checkingRuntime: "Checking managed runtime",
  downloading: "Downloading model",
  installingConverter: "Preparing PTH converter",
  converting: "Converting to ONNX",
  validating: "Validating model",
  testingInference: "Testing neural inference",
  registering: "Registering model",
  assigning: "Assigning voice",
  ready: "Ready",
  cancelled: "Cancelled",
  runtimeRequired: "Runtime required",
  licenseUnverified: "License Unverified",
  incompatible: "Incompatible",
  error: "Error",
};
