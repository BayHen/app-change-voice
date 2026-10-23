export interface AppStatus {
  appName: string;
  version: string;
  platform: string;
  audioEngineReady: boolean;
}

export interface UpdateConfiguration {
  configured: boolean;
  endpoint: string;
  signatureVerificationRequired: boolean;
  allowDowngrades: boolean;
}
