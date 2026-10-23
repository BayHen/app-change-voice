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
}

export interface AIProviderStatus {
  offlineOnly: boolean;
  inferenceBackend: string;
  gpuVerified: boolean;
  activeModelId: string | null;
  fallbackActive: boolean;
  message: string;
}
