export type RestoreMode = "merge" | "replace";
export interface BackupPreview {
  applicationVersion: string;
  createdAt: string;
  presetCount: number;
  profileCount: number;
  soundCount: number;
  presetConflicts: string[];
  profileConflicts: string[];
  portable: boolean;
  warnings: string[];
}
export interface SyncStatus {
  provider: string;
  enabled: boolean;
  state: string;
  lastSyncedAt: string | null;
  message: string;
}
