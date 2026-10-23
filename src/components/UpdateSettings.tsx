import { useEffect, useState } from "react";
import { ArrowClockwise, DownloadSimple, ShieldCheck, WarningCircle } from "@phosphor-icons/react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getUpdateConfiguration } from "../services/tauri";
import type { UpdateConfiguration } from "../types/app";

export function UpdateSettings() {
  const [configuration, setConfiguration] = useState<UpdateConfiguration | null>(null);
  const [update, setUpdate] = useState<Update | null>(null);
  const [busy, setBusy] = useState(false);
  const [installed, setInstalled] = useState(false);
  const [progress, setProgress] = useState(0);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    void getUpdateConfiguration()
      .then((value) => active && setConfiguration(value))
      .catch((reason) => active && setError(String(reason)));
    return () => {
      active = false;
    };
  }, []);

  async function checkNow() {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      const available = await check();
      setUpdate(available);
      setMessage(available ? `Version ${available.version} is available.` : "You are up to date.");
    } catch (reason) {
      setError(`Update check failed: ${String(reason)}`);
    } finally {
      setBusy(false);
    }
  }

  async function install() {
    if (!update) return;
    setBusy(true);
    setError("");
    let downloaded = 0;
    let total = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") downloaded += event.data.chunkLength;
        if (total > 0) setProgress(Math.min(100, Math.round((downloaded / total) * 100)));
      });
      setProgress(100);
      setInstalled(true);
      setMessage("Signed update installed. Restart to use the new version.");
    } catch (reason) {
      setError(`Update installation failed: ${String(reason)}`);
    } finally {
      setBusy(false);
    }
  }

  const configured = configuration?.configured === true;
  return (
    <div className="space-y-4">
      <div className="flex items-start gap-3 rounded-xl border border-white/[.07] bg-black/15 p-4">
        <ShieldCheck className={configured ? "text-emerald-300" : "text-amber-300"} size={22} />
        <div>
          <p className="text-sm font-medium">
            Signed updates: {configured ? "Configured" : "Not configured"}
          </p>
          <p className="mt-1 break-all text-xs leading-5 text-zinc-500">
            {configured
              ? "HTTPS release endpoint and mandatory signature verification are enabled."
              : "This build has no updater public key. Add release signing secrets and rebuild; update checks stay disabled."}
          </p>
        </div>
      </div>
      {update && (
        <div className="rounded-xl bg-cyan-300/[.06] p-4 text-xs text-zinc-300">
          <p className="font-semibold text-cyan-200">VoiceMorph Studio {update.version}</p>
          <p className="mt-2 whitespace-pre-wrap text-zinc-500">
            {update.body || "No release notes provided."}
          </p>
        </div>
      )}
      {progress > 0 && (
        <progress
          className="h-2 w-full accent-cyan-300"
          max={100}
          value={progress}
          aria-label="Update download progress"
        />
      )}
      {error && (
        <p role="alert" className="rounded-lg bg-rose-300/10 p-3 text-xs text-rose-200">
          <WarningCircle className="mr-2 inline" />
          {error}
        </p>
      )}
      {message && (
        <p role="status" className="text-xs text-zinc-400">
          {message}
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          disabled={!configured || busy || installed}
          onClick={() => void checkNow()}
          className="rounded-lg bg-white/10 px-3 py-2 text-xs disabled:cursor-not-allowed disabled:opacity-40"
        >
          <ArrowClockwise className="mr-2 inline" />
          Check for updates
        </button>
        {update && !installed && (
          <button
            type="button"
            disabled={busy}
            onClick={() => void install()}
            className="rounded-lg bg-cyan-300 px-3 py-2 text-xs font-semibold text-cyan-950 disabled:opacity-40"
          >
            <DownloadSimple className="mr-2 inline" />
            Download and install
          </button>
        )}
        {installed && (
          <button
            type="button"
            onClick={() => void relaunch()}
            className="rounded-lg bg-cyan-300 px-3 py-2 text-xs font-semibold text-cyan-950"
          >
            Restart application
          </button>
        )}
      </div>
    </div>
  );
}
