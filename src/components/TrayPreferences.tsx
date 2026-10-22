import { ToggleSwitch } from "./ToggleSwitch";
import type { TrayPreferences as Preferences } from "../types/hotkeys";

export function TrayPreferences({
  value,
  disabled,
  onChange,
}: {
  value: Preferences;
  disabled?: boolean;
  onChange: (value: Preferences) => void;
}) {
  return (
    <div className="space-y-5">
      <ToggleSwitch
        label="Minimize to system tray"
        description="Hide the window when it is minimized."
        checked={value.minimizeToTray}
        disabled={disabled}
        onChange={(minimizeToTray) => onChange({ ...value, minimizeToTray })}
      />
      <ToggleSwitch
        label="Close to system tray"
        description="The window close button hides the app; use Exit from the tray to stop all audio resources."
        checked={value.closeToTray}
        disabled={disabled}
        onChange={(closeToTray) => onChange({ ...value, closeToTray })}
      />
    </div>
  );
}
