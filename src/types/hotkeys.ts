export type HotkeyAction =
  | "toggleMicrophoneMute"
  | "toggleVoiceEffects"
  | "nextVoicePreset"
  | "previousVoicePreset"
  | "toggleHearMyself"
  | "toggleVirtualMicrophone"
  | "toggleRecording";

export interface HotkeyBinding {
  action: HotkeyAction;
  shortcut: string | null;
}
export interface TrayPreferences {
  minimizeToTray: boolean;
  closeToTray: boolean;
}
export interface HotkeyConfiguration {
  schemaVersion: number;
  bindings: HotkeyBinding[];
  tray: TrayPreferences;
}

export interface DesktopActionResult {
  action: HotkeyAction;
  message: string;
}
