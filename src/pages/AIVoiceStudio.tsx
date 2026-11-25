import { AppHeader } from "../components/AppHeader";
import { Badge, Button, Card, EmptyState } from "../components/ui";
import { useEffect, useState } from "react";
import { Cpu, FileArrowUp, ShieldCheck, Sparkle, Trash } from "../components/icons";
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
    <section className="vm-page" aria-label="AI Voice Studio">
      <AppHeader
        title="AI Voice Studio"
        description="Explore a new voice. Keep your identity."
        action={
          <Button
            variant="primary"
            disabled={busy}
            onClick={() => void importModel().catch((error) => setMessage(String(error)))}
          >
            <FileArrowUp size={16} />
            Import ONNX model
          </Button>
        }
      />
      <div className="vm-ai-banner">
        <span className="vm-empty-icon">
          <Sparkle size={25} />
        </span>
        <div>
          <h2>Your next voice, powered locally.</h2>
          <p>Manage trusted models and check runtime compatibility before conversion.</p>
        </div>
        <Badge tone="warning">{busy ? "Checking engine" : "Native inference unavailable"}</Badge>
      </div>
      <div className="vm-lab-grid">
        <div className="vm-lab-main">
          <Card>
            <div className="vm-section-heading">
              <div>
                <h2>Voice model library</h2>
                <p>Verified models, ready for a compatible runtime.</p>
              </div>
              <Badge>{models.length} installed</Badge>
            </div>
            {busy && !models.length ? (
              <p aria-busy="true" className="vm-hint">
                Loading local models…
              </p>
            ) : !models.length ? (
              <EmptyState
                title="A new voice starts with a model"
                description="Import a licensed ONNX model and its manifest. The app verifies its checksum before adding it to your library."
                action={
                  <Button
                    disabled={busy}
                    onClick={() => void importModel().catch((error) => setMessage(String(error)))}
                  >
                    <FileArrowUp size={16} />
                    Import model
                  </Button>
                }
              />
            ) : (
              <div className="vm-ai-model-grid">
                {models.map((model) => (
                  <article className="vm-card" key={model.id}>
                    <span className="vm-preset-art art-violet">
                      <Sparkle size={32} />
                    </span>
                    <div className="vm-section-heading">
                      <h3>{model.name}</h3>
                      <Badge tone={model.integrityVerified ? "success" : "warning"}>
                        {model.integrityVerified ? "Verified" : "Invalid"}
                      </Badge>
                    </div>
                    <p className="vm-hint">{model.description}</p>
                    <p className="vm-hint">
                      {model.architecture} · {model.inputSampleRate / 1000} kHz · {model.license}
                    </p>
                    <p className="vm-hint">{model.statusMessage}</p>
                    <div className="vm-actions">
                      <Button
                        disabled={busy || !model.runtimeAvailable || !model.integrityVerified}
                        onClick={() => void enable(model.id)}
                      >
                        Enable
                      </Button>
                      <Button
                        variant="ghost"
                        aria-label={`Remove ${model.name}`}
                        disabled={busy}
                        onClick={() => {
                          if (window.confirm(`Remove ${model.name} and its local model files?`))
                            void remove(model.id);
                        }}
                      >
                        <Trash size={16} />
                      </Button>
                    </div>
                  </article>
                ))}
              </div>
            )}
          </Card>
          <Card>
            <div className="vm-section-heading">
              <h2>Inference controls</h2>
              <Badge>Runtime required</Badge>
            </div>
            <p className="vm-hint">
              Native conversion is not implemented in this build. Model management works in the
              desktop app; the existing DSP path remains available.
            </p>
            <fieldset disabled className="vm-ai-controls">
              <label className="vm-field">
                Compute provider
                <select aria-label="Compute provider">
                  <option>Unavailable — runtime required</option>
                </select>
              </label>
              <label className="vm-field">
                Pitch shift
                <input type="number" aria-label="AI pitch shift" value={0} readOnly />
              </label>
              <label className="vm-field">
                F0 method
                <select aria-label="F0 method">
                  <option>Unavailable</option>
                </select>
              </label>
            </fieldset>
            <details className="vm-advanced">
              <summary>Advanced inference settings</summary>
              <p className="vm-hint">
                RVC import, F0 extraction, index ratio, chunk size, crossfade, model downloads and
                CPU/GPU selection require a compatible runtime. No conversion settings are applied
                in this build.
              </p>
            </details>
            <Button disabled variant="primary">
              Start AI conversion — unavailable
            </Button>
          </Card>
        </div>
        <aside className="vm-lab-aside">
          <Card>
            <div className="vm-section-heading">
              <h2>
                <Cpu size={17} />
                Runtime status
              </h2>
            </div>
            <dl className="vm-runtime-list">
              <div>
                <dt>Engine</dt>
                <dd>{busy ? "Checking…" : "Native runtime deferred"}</dd>
              </div>
              <div>
                <dt>GPU provider</dt>
                <dd>{provider.gpuVerified ? "Verified" : "Not verified"}</dd>
              </div>
              <div>
                <dt>Active model</dt>
                <dd>{provider.activeModelId ?? "None"}</dd>
              </div>
              <div>
                <dt>Inference latency</dt>
                <dd>Unavailable</dd>
              </div>
              <div>
                <dt>DSP fallback</dt>
                <dd>{provider.fallbackActive ? "Available" : "Inactive"}</dd>
              </div>
            </dl>
            <p className="vm-hint">{provider.message}</p>
          </Card>
          <Card>
            <div className="vm-section-heading">
              <h2>
                <ShieldCheck size={17} />
                Model integrity
              </h2>
            </div>
            <p className="vm-hint">
              Versioned manifests, license validation and SHA-256 checks protect your local model
              library.
            </p>
          </Card>
          <Card>
            <h2 className="font-semibold">Formats & compatibility</h2>
            <p className="vm-hint">ONNX: verified local model storage.</p>
            <p className="vm-hint">RVC / w-okada: no connected native provider in this build.</p>
          </Card>
        </aside>
      </div>
      {message && (
        <p role="status" className="vm-notice">
          {message}
        </p>
      )}
    </section>
  );
}
