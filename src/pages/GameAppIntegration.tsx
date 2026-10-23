import { useEffect, useState } from "react";
import { Broadcast, Plus, Trash, Warning } from "@phosphor-icons/react";
import { AppHeader } from "../components/AppHeader";
import { AudioMeter } from "../components/AudioMeter";
import { DeviceSelector } from "../components/DeviceSelector";
import { StatusIndicator } from "../components/StatusIndicator";
import { useAudioDevices } from "../hooks/useAudioDevices";
import { useAudioRuntime } from "../hooks/useAudioRuntime";
import { usePresets } from "../hooks/usePresets";
import { useVirtualRouting } from "../hooks/useVirtualRouting";
import {
  applyApplicationProfile,
  deleteApplicationProfile,
  getApplicationProfiles,
  saveApplicationProfile,
} from "../services/tauri";
import type { ApplicationKind, ApplicationProfile } from "../types/profiles";

const blank = (presetId = "natural"): ApplicationProfile => ({
  schemaVersion: 1,
  id: `profile-${Date.now()}`,
  name: "New profile",
  application: "other",
  presetId,
  noiseSuppression: true,
  limiterEnabled: true,
  soundboardEnabled: false,
  soundboardVolumePercent: 50,
  clarityPriority: true,
});
export function GameAppIntegration() {
  const devices = useAudioDevices(),
    runtime = useAudioRuntime(),
    routing = useVirtualRouting(),
    presets = usePresets();
  const [profiles, setProfiles] = useState<ApplicationProfile[]>([]),
    [selected, setSelected] = useState(""),
    [editing, setEditing] = useState<ApplicationProfile | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const refresh = () =>
    getApplicationProfiles().then((items) => {
      setProfiles(items);
      setSelected((current) => current || items[0]?.id || "");
    });
  useEffect(() => {
    void getApplicationProfiles()
      .then((items) => {
        setProfiles(items);
        setSelected(items[0]?.id ?? "");
      })
      .catch((e) => setError(String(e)));
  }, []);
  const profile = profiles.find((p) => p.id === selected);
  const inputOptions = (devices.snapshot?.inputDevices ?? []).map((d) => ({
    id: d.id,
    name: d.name,
    kind: "input" as const,
    available: true,
    isDefault: d.isDefault,
  }));
  const virtualOptions = [...routing.devices]
    .sort((a, b) => Number(b.likelyVirtual) - Number(a.likelyVirtual))
    .map((d) => ({
      id: d.id,
      name: d.name,
      kind: "output" as const,
      available: true,
      isDefault: d.isDefault,
    }));
  if (!inputOptions.length)
    inputOptions.push({
      id: "none",
      name: "No microphone",
      kind: "input",
      available: false,
      isDefault: false,
    });
  if (!virtualOptions.length)
    virtualOptions.push({
      id: "none",
      name: "No virtual output",
      kind: "output",
      available: false,
      isDefault: false,
    });
  const meter = Math.max(0, Math.min(100, ((routing.status.outputPeakDbfs + 60) / 60) * 100));
  const active = routing.status.state === "active";
  async function action(task: () => Promise<unknown>) {
    setBusy(true);
    setError("");
    try {
      await task();
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="mx-auto max-w-6xl">
      <AppHeader
        title="Game & App Integration"
        description="One Windows audio pipeline for games, communication apps and recording tools."
        action={
          <StatusIndicator
            label={active ? "Virtual mic active" : "Routing stopped"}
            tone={active ? "online" : routing.status.state === "faulted" ? "error" : "offline"}
          />
        }
      />
      <section className="mt-6 rounded-2xl border border-amber-300/15 bg-amber-300/[.04] p-4 text-xs text-amber-100/75">
        <p className="flex gap-2">
          <Warning />
          Standard Windows audio routing only. VoiceMorph does not inject, hook, inspect game
          memory, modify files or interact with Vanguard.
        </p>
      </section>
      {error && (
        <p role="alert" className="mt-4 rounded-xl bg-rose-300/10 p-3 text-xs text-rose-200">
          {error}
        </p>
      )}
      <div className="mt-5 grid gap-5 lg:grid-cols-2">
        <section className="rounded-2xl border border-white/[.08] bg-[#0f1216] p-5">
          <h2 className="text-sm font-semibold">Audio routing</h2>
          <div className="mt-4 space-y-4">
            <DeviceSelector
              id="integration-input"
              label="Physical microphone"
              value={devices.snapshot?.inputSelection.selectedId ?? "none"}
              devices={inputOptions}
              disabled={devices.refreshing || runtime.status.state === "running"}
              onChange={(id) => void devices.select("input", id)}
            />
            <DeviceSelector
              id="integration-virtual"
              label="Virtual output"
              value={routing.configuration.outputDeviceId ?? "none"}
              devices={virtualOptions}
              disabled={routing.busy || active}
              onChange={(id) => void routing.select(id)}
            />
            <AudioMeter label="Virtual microphone level" level={meter} disabled={!active} />
            <button
              disabled={
                routing.busy ||
                runtime.status.state !== "running" ||
                !routing.configuration.outputDeviceId
              }
              onClick={() => void (active ? routing.stop() : routing.start())}
              className="flex items-center gap-2 rounded-xl bg-cyan-300 px-4 py-2.5 text-sm font-medium text-slate-950 disabled:bg-zinc-800 disabled:text-zinc-600"
            >
              <Broadcast />
              {active ? "Stop routing" : "Start routing"}
            </button>
          </div>
        </section>
        <section className="rounded-2xl border border-white/[.08] bg-[#0f1216] p-5">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold">Application profiles</h2>
            <button
              onClick={() => setEditing(blank(presets.activeId ?? "natural"))}
              className="flex items-center gap-1 text-xs text-cyan-200"
            >
              <Plus />
              Create
            </button>
          </div>
          <select
            aria-label="Application profile"
            value={selected}
            onChange={(e) => setSelected(e.target.value)}
            className="mt-4 w-full rounded-xl border border-white/10 bg-[#090c10] p-3 text-sm"
          >
            {profiles.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          {profile && (
            <div className="mt-4 rounded-xl bg-white/[.025] p-4 text-xs">
              <p className="text-zinc-200">
                {profile.application.toUpperCase()} · preset {profile.presetId}
              </p>
              <p className="mt-2 text-zinc-600">
                Noise {profile.noiseSuppression ? "on" : "off"} · Limiter{" "}
                {profile.limiterEnabled ? "on" : "off"} · Soundboard{" "}
                {profile.soundboardEnabled ? `${profile.soundboardVolumePercent}%` : "off"}
              </p>
              <div className="mt-4 flex gap-2">
                <button
                  disabled={busy}
                  onClick={() => void action(() => applyApplicationProfile(profile.id))}
                  className="rounded-lg bg-cyan-300 px-3 py-2 font-medium text-slate-950"
                >
                  Apply manually
                </button>
                <button
                  onClick={() => setEditing({ ...profile })}
                  className="rounded-lg bg-white/5 px-3 py-2"
                >
                  Edit
                </button>
                <button
                  onClick={() =>
                    confirm(`Delete ${profile.name}?`) &&
                    void action(() => deleteApplicationProfile(profile.id))
                  }
                  className="rounded-lg bg-rose-300/10 px-3 py-2 text-rose-200"
                >
                  <Trash />
                </button>
              </div>
            </div>
          )}
        </section>
      </div>
      <section className="mt-5 rounded-2xl border border-white/[.08] bg-[#0f1216] p-5">
        <h2 className="text-sm font-semibold">Connection guide</h2>
        <div className="mt-4 grid gap-3 text-xs text-zinc-500 sm:grid-cols-2 xl:grid-cols-4">
          {[
            [
              "VALORANT",
              "Voice Chat → Input Device → CABLE Output. Do not change Vanguard or game files.",
            ],
            ["Discord", "Voice & Video → Input Device → CABLE Output; output to headphones."],
            ["OBS Studio", "Add Audio Input Capture and select CABLE Output."],
            [
              "Windows",
              "Sound → Input → CABLE Output only if you intentionally want it system-default.",
            ],
          ].map(([name, text]) => (
            <article key={name} className="rounded-xl bg-white/[.025] p-4">
              <p className="font-medium text-zinc-200">{name}</p>
              <p className="mt-2 leading-5">{text}</p>
            </article>
          ))}
        </div>
      </section>
      {editing && (
        <ProfileEditor
          profile={editing}
          presets={presets.presets.map((p) => ({ id: p.id, name: p.name }))}
          onChange={setEditing}
          onCancel={() => setEditing(null)}
          onSave={() =>
            void action(() => saveApplicationProfile(editing)).then(() => setEditing(null))
          }
        />
      )}
    </div>
  );
}
function ProfileEditor({
  profile,
  presets,
  onChange,
  onCancel,
  onSave,
}: {
  profile: ApplicationProfile;
  presets: { id: string; name: string }[];
  onChange: (p: ApplicationProfile) => void;
  onCancel: () => void;
  onSave: () => void;
}) {
  const field = "rounded-lg border border-white/10 bg-black/25 px-3 py-2 text-sm";
  return (
    <div
      role="dialog"
      aria-modal="true"
      className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4"
    >
      <div className="w-full max-w-xl rounded-2xl border border-white/10 bg-[#12161b] p-6">
        <h2 className="font-semibold">Application profile</h2>
        <div className="mt-5 grid gap-4 sm:grid-cols-2">
          <label className="text-xs text-zinc-500">
            Name
            <input
              className={`mt-2 w-full ${field}`}
              value={profile.name}
              onChange={(e) => onChange({ ...profile, name: e.target.value })}
            />
          </label>
          <label className="text-xs text-zinc-500">
            Application
            <select
              className={`mt-2 w-full ${field}`}
              value={profile.application}
              onChange={(e) =>
                onChange({ ...profile, application: e.target.value as ApplicationKind })
              }
            >
              {["valorant", "discord", "obs", "zoom", "teams", "other"].map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </label>
          <label className="text-xs text-zinc-500">
            Voice preset
            <select
              className={`mt-2 w-full ${field}`}
              value={profile.presetId}
              onChange={(e) => onChange({ ...profile, presetId: e.target.value })}
            >
              {presets.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <label className="text-xs text-zinc-500">
            Soundboard volume
            <input
              type="range"
              min="0"
              max="100"
              className="mt-3 w-full"
              value={profile.soundboardVolumePercent}
              onChange={(e) =>
                onChange({ ...profile, soundboardVolumePercent: Number(e.target.value) })
              }
            />
          </label>
          {[
            ["Noise suppression", "noiseSuppression"],
            ["Output limiter", "limiterEnabled"],
            ["Soundboard", "soundboardEnabled"],
            ["Voice clarity priority", "clarityPriority"],
          ].map(([label, key]) => (
            <label key={key} className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={Boolean(profile[key as keyof ApplicationProfile])}
                onChange={(e) => onChange({ ...profile, [key]: e.target.checked })}
              />
              {label}
            </label>
          ))}
        </div>
        <div className="mt-6 flex justify-end gap-2">
          <button onClick={onCancel} className="px-4 py-2 text-sm text-zinc-500">
            Cancel
          </button>
          <button
            onClick={onSave}
            className="rounded-lg bg-cyan-300 px-4 py-2 text-sm font-medium text-slate-950"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
