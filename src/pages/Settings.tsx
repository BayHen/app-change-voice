import { AppHeader } from "../components/AppHeader";
import { DeviceSelector } from "../components/DeviceSelector";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { HotkeySettings } from "../components/HotkeySettings";
import type { DeviceOption } from "../types/audio";

const noAudioDevices: DeviceOption[] = [
  { id: "unavailable", name: "Audio engine not connected", kind: "input", available: false },
];

export function Settings() {
  return (
    <div className="mx-auto max-w-[960px]">
      <AppHeader
        title="Settings"
        description="Review application preferences and system information. Persistence is planned for a future task."
      />

      <div className="mt-6 space-y-5">
        <SettingsSection title="General" description="Desktop application behavior">
          <ToggleSwitch
            label="Launch at startup"
            description="Coming Soon. This setting is not connected to the operating system."
            checked={false}
            disabled
            onChange={() => undefined}
          />
        </SettingsSection>

        <SettingsSection
          title="Global hotkeys & tray"
          description="System-wide controls and window behavior"
        >
          <HotkeySettings />
        </SettingsSection>

        <SettingsSection title="Audio" description="Default device and processing behavior">
          <div className="grid gap-5 sm:grid-cols-2">
            <DeviceSelector
              id="settings-input-device"
              label="Default input"
              value="unavailable"
              devices={noAudioDevices}
              disabled
              onChange={() => undefined}
            />
            <DeviceSelector
              id="settings-output-device"
              label="Default output"
              value="unavailable"
              devices={noAudioDevices}
              disabled
              onChange={() => undefined}
            />
          </div>
        </SettingsSection>

        <SettingsSection title="Theme" description="Visual appearance">
          <fieldset>
            <legend className="sr-only">Application theme</legend>
            <div className="grid gap-3 sm:grid-cols-3">
              <ThemeOption label="Dark" selected />
              <ThemeOption label="Light" disabled />
              <ThemeOption label="System" disabled />
            </div>
          </fieldset>
        </SettingsSection>

        <SettingsSection title="Application information" description="Build and runtime details">
          <dl className="grid gap-4 text-sm sm:grid-cols-2">
            <InfoRow label="Product" value="VoiceMorph Studio" />
            <InfoRow label="Version" value="0.1.0" mono />
            <InfoRow label="Runtime" value="Tauri 2" />
            <InfoRow label="Target" value="Windows x64" mono />
          </dl>
        </SettingsSection>
      </div>
    </div>
  );
}

interface SettingsSectionProps {
  title: string;
  description: string;
  children: React.ReactNode;
}

function SettingsSection({ title, description, children }: SettingsSectionProps) {
  return (
    <section className="rounded-2xl border border-white/[0.08] bg-[#0f1216] p-5 sm:p-6">
      <div className="border-b border-white/[0.07] pb-4">
        <h2 className="text-sm font-semibold text-zinc-200">{title}</h2>
        <p className="mt-1 text-xs text-zinc-600">{description}</p>
      </div>
      <div className="mt-5 space-y-5">{children}</div>
    </section>
  );
}

function ThemeOption({
  label,
  selected = false,
  disabled = false,
}: {
  label: string;
  selected?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      aria-pressed={selected}
      className={`h-20 rounded-xl border text-sm transition-colors ${
        selected
          ? "border-cyan-300/40 bg-cyan-300/[0.06] text-cyan-200"
          : "border-white/[0.07] bg-white/[0.02] text-zinc-700 disabled:cursor-not-allowed"
      }`}
    >
      {label}
      {disabled ? <span className="mt-1 block text-[10px]">Coming Soon</span> : null}
    </button>
  );
}

function InfoRow({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-white/[0.05] pb-3">
      <dt className="text-xs text-zinc-600">{label}</dt>
      <dd className={`text-xs text-zinc-300 ${mono ? "font-mono" : ""}`}>{value}</dd>
    </div>
  );
}
