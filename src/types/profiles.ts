export type ApplicationKind = "valorant" | "discord" | "obs" | "zoom" | "teams" | "other";
export interface ApplicationProfile {
  schemaVersion: number;
  id: string;
  name: string;
  application: ApplicationKind;
  presetId: string;
  noiseSuppression: boolean;
  limiterEnabled: boolean;
  soundboardEnabled: boolean;
  soundboardVolumePercent: number;
  clarityPriority: boolean;
}
