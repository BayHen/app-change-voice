export type SoundRoute = "headphones" | "virtualMicrophone" | "both";
export interface SoundEntry {
  id: string;
  name: string;
  filePath: string;
  category: string;
  favorite: boolean;
  volumePercent: number;
  route: SoundRoute;
  hotkey: string | null;
  durationSeconds: number;
  missing: boolean;
}
export interface PluginManifest {
  schemaVersion: number;
  id: string;
  name: string;
  version: string;
  apiVersion: string;
  publisher: string;
  trusted: boolean;
  supportedPlatforms: string[];
  requiredCapabilities: string[];
  dependencies: Record<string, string>;
  effects: EffectDescriptor[];
}
export interface EffectDescriptor {
  id: string;
  name: string;
  realtimeSafe: boolean;
  parameters: Array<{
    id: string;
    label: string;
    kind: "float" | "boolean";
    minimum: number | null;
    maximum: number | null;
    default: unknown;
  }>;
}
export interface PluginStatus {
  manifest: PluginManifest;
  enabled: boolean;
}
