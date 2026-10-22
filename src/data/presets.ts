import type { PresetCategory, VoicePresetSummary } from "../types/preset";

export const presetCategories: Array<"All" | PresetCategory> = [
  "All",
  "Clean",
  "Character",
  "Creative",
  "Utility",
];

export const voicePresets: VoicePresetSummary[] = [
  {
    id: "clean-voice",
    name: "Clean Voice",
    description: "Neutral starting point with no tonal transformation.",
    category: "Clean",
    pitch: 0,
    formant: 0,
    available: true,
  },
  {
    id: "broadcast-warmth",
    name: "Broadcast Warmth",
    description: "A controlled low-mid profile for spoken voice.",
    category: "Utility",
    pitch: 0,
    formant: -4,
    available: false,
  },
  {
    id: "bright-narrator",
    name: "Bright Narrator",
    description: "Clear presence intended for narration and streaming.",
    category: "Utility",
    pitch: 1,
    formant: 3,
    available: false,
  },
  {
    id: "deep-character",
    name: "Deep Character",
    description: "Lower pitch and formant for character exploration.",
    category: "Character",
    pitch: -7,
    formant: -12,
    available: false,
  },
  {
    id: "light-character",
    name: "Light Character",
    description: "A lifted tonal profile for expressive dialogue.",
    category: "Character",
    pitch: 5,
    formant: 8,
    available: false,
  },
  {
    id: "digital-shift",
    name: "Digital Shift",
    description: "Experimental color for future creative processing.",
    category: "Creative",
    pitch: 3,
    formant: -2,
    available: false,
  },
];
