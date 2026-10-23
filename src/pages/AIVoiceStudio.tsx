import { useEffect, useState } from "react";
import { Cpu, FileArrowUp, ShieldCheck, Sparkle, Trash } from "@phosphor-icons/react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  addAIModel,
  getAIModels,
  getAIProviderStatus,
  removeAIModel,
  setAIModelEnabled,
} from "../services/tauri";
import type { AIModelStatus, AIProviderStatus } from "../types/ai";

const initialStatus: AIProviderStatus = {
  offlineOnly: true,
  inferenceBackend: "Loading…",
  gpuVerified: false,
  activeModelId: null,
  fallbackActive: true,
  message: "Reading local AI provider status.",
};
const profiles = [
  "Deep AI Voice",
  "Bright AI Voice",
  "Soft AI Voice",
  "Masculine-style Voice",
  "Feminine-style Voice",
  "Synthetic Character Voice",
];

export function AIVoiceStudio() {
  const [models, setModels] = useState<AIModelStatus[]>([]);
  const [provider, setProvider] = useState(initialStatus);
  const [busy, setBusy] = useState(true);
  const [message, setMessage] = useState<string | null>(null);

  async function refresh() {
    setBusy(true);
    try {
      const [nextModels, nextProvider] = await Promise.all([getAIModels(), getAIProviderStatus()]);
      setModels(nextModels);
      setProvider(nextProvider);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    let active = true;
    void Promise.all([getAIModels(), getAIProviderStatus()])
      .then(([nextModels, nextProvider]) => {
        if (!active) return;
        setModels(nextModels);
        setProvider(nextProvider);
      })
      .catch((error: unknown) => {
        if (active) setMessage(String(error));
      })
      .finally(() => {
        if (active) setBusy(false);
      });
    return () => {
      active = false;
    };
  }, []);

  async function importModel() {
    const manifestPath = await open({
      multiple: false,
      filters: [{ name: "AI model manifest", extensions: ["json"] }],
    });
    if (!manifestPath) return;
    const modelPath = await open({
      multiple: false,
      filters: [{ name: "ONNX model", extensions: ["onnx"] }],
    });
    if (!modelPath) return;
    setBusy(true);
    try {
      await addAIModel(manifestPath, modelPath);
      setMessage("Model imported and its SHA-256 was verified.");
      await refresh();
    } catch (error) {
      setMessage(`Import failed: ${String(error)}`);
      setBusy(false);
    }
  }

  async function remove(id: string) {
    setBusy(true);
    try {
      await removeAIModel(id);
      setMessage("Local model files removed.");
      await refresh();
    } catch (error) {
      setMessage(`Remove failed: ${String(error)}`);
      setBusy(false);
    }
  }

  async function enable(id: string) {
    setBusy(true);
    try {
      setProvider(await setAIModelEnabled(id, true));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="mx-auto max-w-6xl" aria-labelledby="ai-title">
      <header className="flex flex-wrap items-end justify-between gap-5">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-violet-300">
            Offline-first
          </p>
          <h1 id="ai-title" className="mt-2 text-3xl font-semibold tracking-tight text-white">
            AI Voice Studio
          </h1>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-zinc-500">
            Manage verified voice-conversion models. AI audio never leaves this computer.
          </p>
        </div>
        <button
          type="button"
          disabled={busy}
          onClick={() => void importModel()}
          className="flex min-h-11 items-center gap-2 rounded-xl bg-violet-300 px-4 text-sm font-semibold text-violet-950 disabled:opacity-40"
        >
          <FileArrowUp size={18} /> Import ONNX model
        </button>
      </header>
      <div className="mt-7 grid gap-4 lg:grid-cols-[1.15fr_0.85fr]">
        <div className="rounded-2xl border border-white/[0.07] bg-[#0d1014] p-5">
          <div className="flex items-center justify-between">
            <h2 className="font-semibold text-zinc-100">Model library</h2>
            <span className="text-xs text-zinc-600">{models.length} installed</span>
          </div>
          {busy && models.length === 0 ? (
            <p className="mt-5 text-sm text-zinc-500">Loading local models…</p>
          ) : null}
          {!busy && models.length === 0 ? (
            <div className="mt-5 rounded-xl border border-dashed border-white/10 p-6 text-center">
              <Sparkle className="mx-auto text-zinc-600" size={28} />
              <p className="mt-3 text-sm font-medium text-zinc-300">
                No trusted ONNX model installed
              </p>
              <p className="mt-1 text-xs leading-5 text-zinc-600">
                Import requires a versioned manifest, allow-listed license and matching SHA-256.
              </p>
            </div>
          ) : null}
          <div className="mt-4 space-y-3">
            {models.map((model) => (
              <article
                key={model.id}
                className="rounded-xl border border-white/[0.07] bg-white/[0.025] p-4"
              >
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="text-sm font-semibold text-zinc-200">{model.name}</h3>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] ${model.integrityVerified ? "bg-emerald-400/10 text-emerald-300" : "bg-red-400/10 text-red-300"}`}
                      >
                        {model.integrityVerified ? "SHA-256 verified" : "Invalid"}
                      </span>
                    </div>
                    <p className="mt-2 text-xs leading-5 text-zinc-500">{model.description}</p>
                    <p className="mt-2 text-[11px] text-zinc-600">
                      {model.architecture} · {model.license} · {model.inputSampleRate / 1000} kHz
                    </p>
                  </div>
                  <button
                    type="button"
                    aria-label={`Remove ${model.name}`}
                    disabled={busy}
                    onClick={() => void remove(model.id)}
                    className="rounded-lg p-2 text-zinc-600 hover:bg-red-400/10 hover:text-red-300 disabled:opacity-40"
                  >
                    <Trash size={17} />
                  </button>
                </div>
                <div className="mt-4 flex items-center justify-between gap-3 border-t border-white/[0.06] pt-3">
                  <p className="text-[11px] text-zinc-600">{model.statusMessage}</p>
                  <button
                    type="button"
                    disabled={busy || !model.runtimeAvailable || !model.integrityVerified}
                    onClick={() => void enable(model.id)}
                    className="shrink-0 rounded-lg border border-white/10 px-3 py-1.5 text-xs text-zinc-300 disabled:cursor-not-allowed disabled:opacity-35"
                  >
                    Enable
                  </button>
                </div>
              </article>
            ))}
          </div>
        </div>
        <aside className="space-y-4">
          <div className="rounded-2xl border border-white/[0.07] bg-[#0d1014] p-5">
            <div className="flex items-center gap-2">
              <Cpu size={19} className="text-violet-300" />
              <h2 className="font-semibold text-zinc-100">Runtime status</h2>
            </div>
            <dl className="mt-5 space-y-3 text-sm">
              <div className="flex justify-between gap-4">
                <dt className="text-zinc-600">Mode</dt>
                <dd className="text-zinc-300">Offline prototype</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-zinc-600">Backend</dt>
                <dd className="text-right text-zinc-300">{provider.inferenceBackend}</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-zinc-600">GPU</dt>
                <dd className="text-amber-300">
                  {provider.gpuVerified ? "Verified" : "Not verified"}
                </dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-zinc-600">Realtime</dt>
                <dd className="text-amber-300">Not available</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-zinc-600">Fallback</dt>
                <dd className="text-emerald-300">DSP active</dd>
              </div>
            </dl>
            <p className="mt-4 border-t border-white/[0.06] pt-4 text-xs leading-5 text-zinc-500">
              {provider.message}
            </p>
          </div>
          <div className="rounded-2xl border border-violet-300/15 bg-violet-300/[0.04] p-5">
            <div className="flex items-center gap-2 text-violet-200">
              <ShieldCheck size={19} />
              <h2 className="font-semibold">Privacy & consent</h2>
            </div>
            <p className="mt-3 text-xs leading-5 text-zinc-500">
              Only use voices and reference recordings you own or have permission to process.
              Celebrity or person-specific impersonation is not provided.
            </p>
          </div>
          {message ? (
            <div
              role="status"
              className="rounded-xl border border-amber-300/15 bg-amber-300/[0.04] p-3 text-xs text-amber-200"
            >
              {message}
            </div>
          ) : null}
        </aside>
      </div>
      <div className="mt-4 rounded-2xl border border-white/[0.07] bg-[#0d1014] p-5">
        <h2 className="font-semibold text-zinc-100">Voice profiles</h2>
        <div className="mt-4 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {profiles.map((profile) => (
            <button
              key={profile}
              type="button"
              disabled
              className="min-h-12 rounded-xl border border-white/[0.06] bg-white/[0.02] px-3 text-left text-sm text-zinc-600 disabled:cursor-not-allowed"
            >
              {profile}
              <span className="float-right text-[10px] uppercase">model required</span>
            </button>
          ))}
        </div>
        <p className="mt-4 text-xs text-zinc-600">
          A/B comparison, recording test, quality mode and latency estimates remain disabled until a
          compatible runtime passes validation.
        </p>
      </div>
    </section>
  );
}
