import { useEffect, useMemo, useState } from "react";
import {
  getHotkeyConfiguration,
  resetHotkeyConfiguration,
  setHotkeyConfiguration,
} from "../services/tauri";
import type { HotkeyAction, HotkeyConfiguration } from "../types/hotkeys";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { TrayPreferences } from "./TrayPreferences";

const labels: Record<HotkeyAction, string> = {
  toggleMicrophoneMute: "Toggle microphone mute",
  toggleVoiceEffects: "Toggle voice effects",
  nextVoicePreset: "Next voice preset",
  previousVoicePreset: "Previous voice preset",
  toggleHearMyself: "Toggle Hear Myself",
  toggleVirtualMicrophone: "Toggle virtual microphone",
  toggleRecording: "Start / stop recording",
};
const errorMessage = (error: unknown) =>
  error instanceof Error
    ? error.message
    : typeof error === "object" && error && "message" in error
      ? String(error.message)
      : String(error);

export function HotkeySettings() {
  const [configuration, setConfiguration] = useState<HotkeyConfiguration | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    void getHotkeyConfiguration()
      .then((value) => active && setConfiguration(value))
      .catch((reason) => active && setError(errorMessage(reason)))
      .finally(() => active && setLoading(false));
    return () => {
      active = false;
    };
  }, []);
  const duplicates = useMemo(() => {
    const counts = new Map<string, number>();
    for (const binding of configuration?.bindings ?? [])
      if (binding.shortcut)
        counts.set(
          binding.shortcut.toLowerCase(),
          (counts.get(binding.shortcut.toLowerCase()) ?? 0) + 1,
        );
    return new Set([...counts].filter(([, count]) => count > 1).map(([value]) => value));
  }, [configuration]);
  const updateShortcut = (action: HotkeyAction, shortcut: string | null) =>
    setConfiguration((current) =>
      current
        ? {
            ...current,
            bindings: current.bindings.map((binding) =>
              binding.action === action ? { ...binding, shortcut } : binding,
            ),
          }
        : current,
    );
  const save = async () => {
    if (!configuration || duplicates.size) return;
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      setConfiguration(await setHotkeyConfiguration(configuration));
      setMessage("Hotkeys registered and saved.");
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setSaving(false);
    }
  };
  const reset = async () => {
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      setConfiguration(await resetHotkeyConfiguration());
      setMessage("Default shortcuts restored.");
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setSaving(false);
    }
  };
  if (loading || !configuration)
    return (
      <div
        className="h-32 animate-pulse rounded-xl bg-white/[0.025]"
        aria-label="Loading hotkeys"
      />
    );
  return (
    <div className="space-y-5">
      <div className="space-y-2">
        {configuration.bindings.map((binding) => (
          <ShortcutRecorder
            key={binding.action}
            label={labels[binding.action]}
            value={binding.shortcut}
            disabled={saving}
            duplicate={!!binding.shortcut && duplicates.has(binding.shortcut.toLowerCase())}
            onChange={(value) => updateShortcut(binding.action, value)}
          />
        ))}
      </div>
      {duplicates.size ? (
        <p role="alert" className="text-xs text-rose-300">
          Duplicate shortcuts must be resolved before saving.
        </p>
      ) : null}
      {error ? (
        <p role="alert" className="text-xs text-rose-300">
          {error}
        </p>
      ) : null}
      {message ? (
        <p role="status" className="text-xs text-emerald-300">
          {message}
        </p>
      ) : null}
      <div className="flex gap-3">
        <button
          type="button"
          disabled={saving || duplicates.size > 0}
          onClick={() => void save()}
          className="h-10 rounded-xl bg-cyan-300 px-4 text-sm font-medium text-[#071014] disabled:opacity-40"
        >
          {saving ? "Saving…" : "Save hotkeys"}
        </button>
        <button
          type="button"
          disabled={saving}
          onClick={() => void reset()}
          className="h-10 rounded-xl border border-white/[0.09] px-4 text-sm text-zinc-400"
        >
          Reset defaults
        </button>
      </div>
      <div className="border-t border-white/[0.07] pt-5">
        <TrayPreferences
          value={configuration.tray}
          disabled={saving}
          onChange={(tray) => setConfiguration({ ...configuration, tray })}
        />
      </div>
      <p className="text-[11px] leading-5 text-zinc-600">
        Shortcuts require a modifier key. Windows-reserved combinations are rejected. If another
        application already owns a shortcut, saving fails and the previous registrations remain
        active.
      </p>
    </div>
  );
}
