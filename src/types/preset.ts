export type PresetCategory = "Clean" | "Character" | "Creative" | "Utility";

export interface VoicePresetSummary {
  id: string;
  name: string;
  description: string;
  category: PresetCategory;
  pitch: number;
  formant: number;
  available: boolean;
}
