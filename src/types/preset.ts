export type PresetCategory = "Natural" | "Character" | "Creative" | "Utility";

export type PresetEffect =
  | { type: "ringModulator"; frequencyHz: number; mix: number }
  | { type: "radioFilter"; highPassHz: number; lowPassHz: number; drive: number; mix: number }
  | { type: "tremolo"; rateHz: number; depth: number };

export interface VoicePresetSummary {
  schemaVersion: number;
  id: string;
  name: string;
  description: string;
  category: PresetCategory;
  pitchSemitones: number;
  formantSemitones: number;
  gain: { makeupDb: number };
  effects: PresetEffect[];
  builtIn: boolean;
}
