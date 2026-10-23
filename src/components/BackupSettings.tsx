import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  ArrowCounterClockwise,
  CloudSlash,
  DownloadSimple,
  UploadSimple,
  Warning,
} from "@phosphor-icons/react";
import * as api from "../services/tauri";
import type { BackupPreview, RestoreMode, SyncStatus } from "../types/backup";

export function BackupSettings() {
  const [status, setStatus] = useState<SyncStatus | null>(null),
    [preview, setPreview] = useState<BackupPreview | null>(null),
    [restorePath, setRestorePath] = useState(""),
    [busy, setBusy] = useState(false),
    [message, setMessage] = useState(""),
    [error, setError] = useState("");
  useEffect(() => {
    void api
      .getSyncStatus()
      .then(setStatus)
      .catch((e) => setError(String(e)));
  }, []);
  async function run(task: () => Promise<unknown>, success: string) {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await task();
      setMessage(success);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function chooseBackup() {
    const path = await open({
      multiple: false,
      filters: [{ name: "VoiceMorph backup", extensions: ["json", "vms-backup"] }],
    });
    if (typeof path !== "string") return;
    setBusy(true);
    setError("");
    try {
      setPreview(await api.previewBackup(path));
      setRestorePath(path);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function restore(mode: RestoreMode) {
    await run(
      () => api.restoreBackup(restorePath, mode),
      `Backup restored in ${mode} mode. Restart is recommended.`,
    );
    setPreview(null);
  }
  return (
    <div className="space-y-5">
      <div className="grid gap-3 sm:grid-cols-2">
        <Action
          title="Backup Now"
          detail="Settings, custom presets, profiles and portable soundboard metadata."
          icon={<DownloadSimple />}
          disabled={busy}
          onClick={() =>
            void save({
              defaultPath: `voicemorph-backup-${Date.now()}.vms-backup`,
              filters: [{ name: "VoiceMorph backup", extensions: ["vms-backup", "json"] }],
            }).then((path) => {
              if (path) return run(() => api.exportAllSettings(path), "Backup exported.");
            })
          }
        />
        <Action
          title="Restore Backup"
          detail="Validate and preview before merge or replace."
          icon={<ArrowCounterClockwise />}
          disabled={busy}
          onClick={() => void chooseBackup()}
        />
        <Action
          title="Export All Presets"
          detail="Portable custom presets; built-ins are not duplicated."
          icon={<UploadSimple />}
          disabled={busy}
          onClick={() =>
            void save({
              defaultPath: "voicemorph-presets.json",
              filters: [{ name: "Preset pack", extensions: ["json"] }],
            }).then((path) => {
              if (path) return run(() => api.exportAllPresets(path), "Preset pack exported.");
            })
          }
        />
        <Action
          title="Import All Presets"
          detail="Merge portable presets by stable ID."
          icon={<DownloadSimple />}
          disabled={busy}
          onClick={() =>
            void open({
              multiple: false,
              filters: [{ name: "Preset pack", extensions: ["json"] }],
            }).then((path) => {
              if (typeof path === "string")
                return run(() => api.importAllPresets(path, "merge"), "Preset pack imported.");
            })
          }
        />
      </div>
      {error && (
        <p role="alert" className="rounded-xl bg-rose-300/10 p-3 text-xs text-rose-200">
          {error}
        </p>
      )}
      {message && (
        <p role="status" className="rounded-xl bg-emerald-300/10 p-3 text-xs text-emerald-200">
          {message}
        </p>
      )}
      <div className="rounded-xl border border-white/[.07] bg-black/15 p-4">
        <div className="flex items-center gap-2">
          <CloudSlash className="text-zinc-500" />
          <p className="text-sm font-medium">Sync Settings</p>
        </div>
        <p className="mt-2 text-xs leading-5 text-zinc-500">
          {status?.message ?? "Loading sync status…"}
        </p>
        <p className="mt-2 text-[11px] uppercase tracking-wider text-zinc-700">
          Status: {status?.state ?? "loading"} · Provider: {status?.provider ?? "—"}
        </p>
      </div>
      {preview && (
        <div
          role="dialog"
          aria-modal="true"
          className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4"
        >
          <div className="w-full max-w-lg rounded-2xl border border-white/10 bg-[#12161b] p-6">
            <h3 className="font-semibold">Backup preview</h3>
            <dl className="mt-4 grid grid-cols-3 gap-3 text-center">
              <Count label="Presets" value={preview.presetCount} />
              <Count label="Profiles" value={preview.profileCount} />
              <Count label="Sounds" value={preview.soundCount} />
            </dl>
            <p className="mt-4 text-xs text-zinc-500">
              Created {preview.createdAt} · App {preview.applicationVersion}
            </p>
            {preview.presetConflicts.length + preview.profileConflicts.length > 0 && (
              <p className="mt-3 flex gap-2 rounded-lg bg-amber-300/10 p-3 text-xs text-amber-200">
                <Warning />
                Conflicts: {[...preview.presetConflicts, ...preview.profileConflicts].join(", ")}
              </p>
            )}
            <p className="mt-3 text-xs text-zinc-600">
              Merge keeps unrelated entries. Replace writes a verified safety backup first.
            </p>
            <div className="mt-6 flex flex-wrap justify-end gap-2">
              <button onClick={() => setPreview(null)} className="px-3 py-2 text-sm text-zinc-500">
                Cancel
              </button>
              <button
                onClick={() => void restore("merge")}
                className="rounded-lg bg-white/10 px-3 py-2 text-sm"
              >
                Merge
              </button>
              <button
                onClick={() => void restore("replace")}
                className="rounded-lg bg-rose-300 px-3 py-2 text-sm font-medium text-rose-950"
              >
                Replace
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
function Action({
  title,
  detail,
  icon,
  disabled,
  onClick,
}: {
  title: string;
  detail: string;
  icon: React.ReactNode;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      disabled={disabled}
      onClick={onClick}
      className="flex items-start gap-3 rounded-xl border border-white/[.07] bg-black/15 p-4 text-left hover:border-cyan-300/20 disabled:opacity-40"
    >
      <span className="mt-0.5 text-cyan-300">{icon}</span>
      <span>
        <span className="block text-sm text-zinc-200">{title}</span>
        <span className="mt-1 block text-xs leading-5 text-zinc-600">{detail}</span>
      </span>
    </button>
  );
}
function Count({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-lg bg-white/[.035] p-3">
      <dt className="text-[11px] text-zinc-600">{label}</dt>
      <dd className="mt-1 font-mono text-zinc-200">{value}</dd>
    </div>
  );
}
