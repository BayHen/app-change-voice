import { useState, type ReactNode } from "react";
import { AppHeader } from "../components/AppHeader";
import { Card, Button } from "../components/ui";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { HotkeySettings } from "../components/HotkeySettings";
import { DiscordIntegration } from "../components/DiscordIntegration";
import { BackupSettings } from "../components/BackupSettings";
import { AudioDiagnostics } from "../components/AudioDiagnostics";
import { UpdateSettings } from "../components/UpdateSettings";
import { useAppStatus } from "../hooks/useAppStatus";
import { AIVoiceStudio } from "./AIVoiceStudio";

export function Settings({
  advanced,
  effects,
  studio,
  routing,
  integrations,
}: {
  advanced?: ReactNode;
  effects?: ReactNode;
  studio?: ReactNode;
  routing?: ReactNode;
  integrations?: ReactNode;
}) {
  const [tab, setTab] = useState("General");
  const tabs = [
    "General",
    "Appearance",
    "Audio devices",
    "Voice presets",
    "DSP controls",
    "Virtual microphone",
    "AI runtime",
    "Hardware acceleration",
    "Performance",
    "Keyboard shortcuts",
    "Storage",
    "Updates",
    "About",
    "Games & apps",
  ];
  const appStatus = useAppStatus();
  return (
    <div className="vm-page vm-settings">
      <AppHeader
        title="Settings"
        description="Review persistent preferences, release updates and system information."
      />

      <div className="vm-settings-tabs" role="group" aria-label="Settings categories">
        {tabs.map((item) => (
          <button type="button" key={item} aria-pressed={tab === item} onClick={() => setTab(item)}>
            {item}
          </button>
        ))}
      </div>
      {tab !== "General" ? (
        <div className="vm-settings-detail">
          {tab === "Audio devices" ? (
            advanced
          ) : tab === "Voice presets" ? (
            effects
          ) : tab === "DSP controls" ? (
            studio
          ) : tab === "Virtual microphone" ? (
            routing
          ) : tab === "AI runtime" ? (
            <AIVoiceStudio />
          ) : tab === "Performance" ? (
            <AudioDiagnostics />
          ) : tab === "Keyboard shortcuts" ? (
            <HotkeySettings />
          ) : tab === "Storage" ? (
            <BackupSettings />
          ) : tab === "Updates" ? (
            <UpdateSettings />
          ) : tab === "Appearance" ? (
            <SettingsSection
              title="Appearance"
              description="The studio theme is shared by every workspace."
            >
              <div className="grid grid-cols-3 gap-3">
                <ThemeOption label="Dark" selected />
                <ThemeOption label="Light" disabled />
                <ThemeOption label="System" disabled />
              </div>
            </SettingsSection>
          ) : tab === "Hardware acceleration" ? (
            <AIVoiceStudio />
          ) : tab === "About" ? (
            <SettingsSection
              title="VoiceMorph Studio"
              description="Local real-time audio workspace"
            >
              <p className="vm-hint">
                Version {appStatus.status?.version ?? "Unavailable"} · Tauri 2 · Windows x64
              </p>
            </SettingsSection>
          ) : (
            integrations
          )}
        </div>
      ) : (
        <div className="vm-settings-content mt-6 space-y-5">
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

          <SettingsSection
            title="Discord Integration"
            description="Route processed voice through VB-CABLE"
          >
            <DiscordIntegration />
          </SettingsSection>

          <SettingsSection title="Backup & Sync" description="Portable local backup and restore">
            <BackupSettings />
          </SettingsSection>

          <SettingsSection title="Audio" description="Default device and processing behavior">
            <p className="vm-hint">
              Choose your input and output, review driver-reported sample rates and buffer
              capabilities, and check the signal path.
            </p>
            <Button onClick={() => setTab("Audio devices")}>Configure audio devices</Button>
          </SettingsSection>

          <SettingsSection
            title="Audio Diagnostics"
            description="Runtime health, performance counters and privacy-safe report export"
          >
            <AudioDiagnostics />
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
              <InfoRow label="Version" value={appStatus.status?.version ?? "Loading…"} mono />
              <InfoRow label="Runtime" value="Tauri 2" />
              <InfoRow label="Target" value="Windows x64" mono />
            </dl>
          </SettingsSection>

          <SettingsSection
            title="Application Updates"
            description="Signed releases from the official HTTPS endpoint"
          >
            <UpdateSettings />
          </SettingsSection>
        </div>
      )}
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
    <Card>
      <div className="border-b border-white/[0.07] pb-4">
        <h2 className="text-sm font-semibold text-zinc-200">{title}</h2>
        <p className="mt-1 text-xs text-zinc-600">{description}</p>
      </div>
      <div className="mt-5 space-y-5">{children}</div>
    </Card>
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
