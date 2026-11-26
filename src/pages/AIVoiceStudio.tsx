import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  BrainCircuit,
  FileUp,
  FolderOpen,
  Play,
  Square,
  ShieldCheck,
  Trash2,
  RefreshCw,
  Cpu,
} from "lucide-react";
import { AppHeader } from "../components/AppHeader";
import { Badge, Button, Card, EmptyState, Slider, StudioDialog } from "../components/ui";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { useAudioSession } from "../hooks/AudioSession";
import { addAIModel, getAIModels, removeAIModel } from "../services/tauri";
import type { AIEngineConfig, AIModelStatus } from "../types/ai";

const time = (value: number | null | undefined) =>
  value == null ? "Unavailable" : `${value.toFixed(1)} ms`;
export function AIVoiceStudio() {
  const { ai } = useAudioSession();
  const status = ai.status;
  const [models, setModels] = useState<AIModelStatus[]>([]),
    [selectedId, setSelectedId] = useState("");
  const [localDraft, setDraft] = useState<AIEngineConfig | null>(null),
    [dirty, setDirty] = useState(false);
  const draft = localDraft ?? status?.config ?? null;
  const [busy, setBusy] = useState(false),
    [message, setMessage] = useState<string | null>(null);
  const [importOpen, setImportOpen] = useState(false),
    [rights, setRights] = useState(false);
  const [officialOpen, setOfficialOpen] = useState(false),
    [officialRights, setOfficialRights] = useState(false);
  const [runtimeOpen, setRuntimeOpen] = useState(false),
    [runtimePath, setRuntimePath] = useState("");
  const [digest, setDigest] = useState(""),
    [trusted, setTrusted] = useState(false);
  const active = !!status && ["starting", "ready", "converting"].includes(status.state);
  const selected = models.find((model) => model.id === selectedId);
  const compatible =
    selected?.backend === "w-okada-rvc" && selected.integrityVerified && selected.consentConfirmed;
  useEffect(() => {
    let mounted = true;
    void getAIModels()
      .then((value) => {
        if (mounted) {
          setModels(value);
          setSelectedId(value.find((m) => m.backend === "w-okada-rvc")?.id ?? "");
        }
      })
      .catch((error) => {
        if (mounted) setMessage(String(error));
      });
    return () => {
      mounted = false;
    };
  }, []);
  async function refresh() {
    try {
      setModels(await getAIModels());
      await ai.refresh();
    } catch (error) {
      setMessage(String(error));
    }
  }
  function update<K extends keyof AIEngineConfig>(key: K, value: AIEngineConfig[K]) {
    if (draft) {
      setDraft({ ...draft, [key]: value });
      setDirty(true);
    }
  }
  async function importModel() {
    setBusy(true);
    setMessage(null);
    try {
      const manifest = await open({
        multiple: false,
        filters: [{ name: "Voice model manifest", extensions: ["json"] }],
      });
      if (typeof manifest !== "string") return;
      const model = await open({
        multiple: false,
        filters: [{ name: "RVC ONNX model", extensions: ["onnx"] }],
      });
      if (typeof model !== "string") return;
      const imported = await addAIModel(manifest, model, rights);
      await refresh();
      setSelectedId(imported.id);
      setImportOpen(false);
      setMessage(
        "Model imported; checksum and declared license validated. Runtime compatibility is checked on load.",
      );
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }
  async function install() {
    const result = await ai.install(runtimePath, digest.trim(), trusted);
    if (result) {
      setRuntimeOpen(false);
      setMessage("Verified runtime installed. Classic DSP remains available offline.");
      await refresh();
    }
  }
  async function save() {
    if (!draft) return;
    const result = await ai.save(draft);
    if (result) {
      setDraft(result.config);
      setDirty(false);
      setMessage("AI preferences saved. Load a model to apply them.");
    }
  }
  return (
    <section className="vm-page" aria-label="AI Voice Studio">
      <AppHeader
        title="AI Voice Studio"
        description="A local AI voice, inside your existing studio."
        action={
          <Button
            variant="primary"
            disabled={busy}
            onClick={() => {
              setRights(false);
              setImportOpen(true);
            }}
          >
            <FileUp size={16} />
            Import ONNX model
          </Button>
        }
      />
      <div className="vm-ai-banner">
        <span className="vm-empty-icon">
          <BrainCircuit size={26} />
        </span>
        <div>
          <h2>
            {!status
              ? "Voice engine status unavailable"
              : status.mode === "wOkada"
                ? status.state === "converting"
                  ? "AI conversion active"
                  : status.state === "ready"
                    ? "AI model ready. Waiting for audio."
                    : status.classicFallbackActive
                      ? "Classic DSP fallback active"
                      : "AI selected. Your voice stays muted until ready."
                : "Classic DSP is your current voice."}
          </h2>
          <p>Capture, monitoring, soundboard mixing and virtual output stay in VoiceMorph.</p>
        </div>
        <Badge
          tone={
            status?.state === "faulted"
              ? "warning"
              : status?.state === "converting"
                ? "success"
                : "neutral"
          }
        >
          {status ? status.state : "Desktop bridge unavailable"}
        </Badge>
      </div>
      <div className="vm-lab-grid">
        <div className="vm-lab-main">
          <Card>
            <div className="vm-section-heading">
              <div>
                <h2>Voice mode</h2>
                <p>Switching to Classic DSP makes your Classic voice audible again.</p>
              </div>
            </div>
            <div className="vm-actions">
              <Button
                aria-pressed={status?.mode === "classicDsp"}
                disabled={!status || (ai.busy && status.mode === "classicDsp")}
                onClick={() => void ai.classic()}
              >
                Use Classic DSP
              </Button>
              <Button
                variant="danger"
                disabled={!status || status.mode !== "wOkada"}
                onClick={() => void ai.stop()}
              >
                <Square size={14} />
                Stop AI, keep voice muted
              </Button>
            </div>
          </Card>
          <Card>
            <div className="vm-section-heading">
              <div>
                <h2>Voice model library</h2>
                <p>RVC ONNX models with acknowledged voice rights.</p>
              </div>
              <Button variant="ghost" aria-label="Refresh AI models" onClick={() => void refresh()}>
                <RefreshCw size={16} />
              </Button>
            </div>
            <div className="vm-actions">
              <Button
                disabled={
                  !status?.runtimeInstalled ||
                  ai.busy ||
                  active ||
                  models.some((m) => m.id === "tsukuyomi-official-normal1")
                }
                onClick={() => {
                  setOfficialRights(false);
                  setOfficialOpen(true);
                }}
              >
                Get official Tsukuyomi-chan voice
              </Button>
            </div>
            {models.length === 0 ? (
              <EmptyState
                title="Choose a voice you have permission to use"
                description="Import an RVC ONNX model with its licensed manifest. Voice checkpoints, archives and unverified downloads are not loaded."
              />
            ) : (
              <div className="vm-ai-model-grid">
                {models.map((model) => (
                  <article key={model.id} className="vm-card">
                    <button
                      className="vm-ai-model-choice"
                      type="button"
                      aria-pressed={selectedId === model.id}
                      onClick={() => setSelectedId(model.id)}
                    >
                      <span className="vm-preset-art">
                        <BrainCircuit size={32} />
                      </span>
                      <strong>{model.name}</strong>
                    </button>
                    <p className="vm-hint">
                      {model.architecture} · {model.license} · {model.inputSampleRate / 1000} kHz
                      transport
                    </p>
                    <Badge tone={model.integrityVerified ? "success" : "warning"}>
                      {model.integrityVerified ? "Checksum verified" : "Integrity failure"}
                    </Badge>
                    {model.attribution && <p className="vm-hint">{model.attribution}</p>}
                    {model.licenseUrl && (
                      <a
                        className="vm-hint"
                        href={model.licenseUrl}
                        target="_blank"
                        rel="noreferrer"
                      >
                        Voice/model terms
                      </a>
                    )}
                    <p className="vm-hint">
                      {model.backend === "w-okada-rvc"
                        ? model.statusMessage
                        : "Storage only: this model is not an RVC provider model."}
                    </p>
                    <Button
                      variant="ghost"
                      disabled={busy || (status?.modelId === model.id && active)}
                      aria-label={`Remove ${model.name}`}
                      onClick={() => {
                        if (!confirm(`Delete ${model.name} and its local model files?`)) return;
                        setBusy(true);
                        void removeAIModel(model.id)
                          .then(refresh)
                          .catch((error) => setMessage(String(error)))
                          .finally(() => setBusy(false));
                      }}
                    >
                      <Trash2 size={15} />
                      Remove model
                    </Button>
                  </article>
                ))}
              </div>
            )}
            <div className="vm-toolbar">
              <label className="vm-field">
                Selected voice
                <select
                  aria-label="Selected AI voice"
                  value={selectedId}
                  disabled={active || models.length === 0}
                  onChange={(e) => setSelectedId(e.target.value)}
                >
                  <option value="">Choose RVC ONNX model</option>
                  {models.map((model) => (
                    <option key={model.id} value={model.id}>
                      {model.name}
                    </option>
                  ))}
                </select>
              </label>
              <Button
                variant="primary"
                disabled={!compatible || !status?.runtimeInstalled || ai.busy || active || dirty}
                onClick={() => void ai.start(selectedId)}
              >
                <Play size={15} />
                Load & start AI
              </Button>
            </div>
            {dirty && <p className="vm-hint">Apply your settings before loading the model.</p>}
          </Card>
          <Card>
            <div className="vm-section-heading">
              <h2>Voice conversion settings</h2>
              <Badge>{active ? "Stop AI to edit" : "Saved locally"}</Badge>
            </div>
            {draft ? (
              <fieldset disabled={active || ai.busy || !status}>
                <div className="vm-ai-controls">
                  <label className="vm-field">
                    Compute preference
                    <select
                      aria-label="AI compute preference"
                      value={draft.provider}
                      onChange={(e) =>
                        update("provider", e.target.value as AIEngineConfig["provider"])
                      }
                    >
                      <option value="auto">Auto: compatible CUDA or CPU</option>
                      <option value="cpu">CPU</option>
                      <option value="cuda">NVIDIA CUDA</option>
                    </select>
                  </label>
                  <label className="vm-field">
                    F0 estimation
                    <select
                      aria-label="F0 estimation"
                      value={draft.f0Method}
                      onChange={(e) =>
                        update("f0Method", e.target.value as AIEngineConfig["f0Method"])
                      }
                    >
                      <option value="dio">DIO</option>
                      <option value="harvest">Harvest</option>
                      <option value="rmvpe_onnx">RMVPE ONNX (runtime asset required)</option>
                    </select>
                  </label>
                  <label className="vm-field">
                    Chunk size
                    <select
                      aria-label="AI chunk size"
                      value={draft.chunkFrames}
                      onChange={(e) => {
                        const frames = Number(e.target.value);
                        setDraft({
                          ...draft,
                          chunkFrames: frames,
                          crossfadeFrames: Math.min(frames, draft.crossfadeFrames),
                        });
                        setDirty(true);
                      }}
                    >
                      {[2048, 4096, 8192, 16384].map((frames) => (
                        <option key={frames} value={frames}>
                          {frames} frames · {(frames / 48).toFixed(1)} ms
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
                <Slider
                  label="AI pitch shift"
                  min={-12}
                  max={12}
                  value={draft.pitchSemitones}
                  unit=" st"
                  onChange={(v) => update("pitchSemitones", v)}
                />
                <details className="vm-advanced">
                  <summary>Advanced inference & failure policy</summary>
                  <div className="vm-ai-controls">
                    <label className="vm-field">
                      Crossfade frames
                      <input
                        aria-label="AI crossfade frames"
                        type="number"
                        min={128}
                        max={Math.min(4096, draft.chunkFrames)}
                        value={draft.crossfadeFrames}
                        onChange={(e) => update("crossfadeFrames", Number(e.target.value))}
                      />
                    </label>
                    <label className="vm-field">
                      Extra context frames
                      <input
                        aria-label="AI extra context"
                        type="number"
                        min={0}
                        max={32768}
                        value={draft.extraContextFrames}
                        onChange={(e) => update("extraContextFrames", Number(e.target.value))}
                      />
                    </label>
                    <label className="vm-field">
                      CPU thread limit
                      <input
                        aria-label="AI CPU threads"
                        type="number"
                        min={1}
                        max={8}
                        value={draft.cpuThreads}
                        onChange={(e) => update("cpuThreads", Number(e.target.value))}
                      />
                    </label>
                    <label className="vm-field">
                      Silence threshold
                      <input
                        aria-label="AI silence threshold"
                        type="number"
                        min={0}
                        max={0.1}
                        step={0.00001}
                        value={draft.noiseThreshold}
                        onChange={(e) => update("noiseThreshold", Number(e.target.value))}
                      />
                    </label>
                  </div>
                  <ToggleSwitch
                    label="Allow CPU fallback for unavailable CUDA"
                    checked={draft.allowCpuFallback}
                    onChange={(v) => update("allowCpuFallback", v)}
                  />
                  <ToggleSwitch
                    label="Allow Classic DSP on AI failure"
                    description="Your Classic voice may be heard if AI stops responding. Leave off to keep the voice muted."
                    checked={draft.failurePolicy === "classicDsp"}
                    onChange={(v) => update("failurePolicy", v ? "classicDsp" : "mute")}
                  />
                  <p className="vm-hint">
                    Index ratio remains 0 without a validated index. Formant shaping is available in
                    Classic DSP. Latency/quality presets need measured model benchmarks.
                  </p>
                </details>
                <Button disabled={!dirty} onClick={() => void save()}>
                  Apply AI settings
                </Button>
              </fieldset>
            ) : (
              <p className="vm-hint">
                Connect the desktop backend to load editable AI preferences.
              </p>
            )}
          </Card>
        </div>
        <aside className="vm-lab-aside">
          <Card>
            <div className="vm-section-heading">
              <h2>
                <Cpu size={17} />
                Managed AI runtime
              </h2>
              <Badge tone={status?.runtimeInstalled ? "success" : "warning"}>
                {!status ? "Unavailable" : status.runtimeInstalled ? "Installed" : "Not installed"}
              </Badge>
            </div>
            <p className="vm-hint">
              Install Python, CUDA-compatible inference dependencies and HuBERT together. No
              terminal, system Python, driver installation or separate browser is needed.
            </p>
            <p className="vm-hint">
              Download:{" "}
              {status
                ? (status.recommendedDownloadBytes / 1024 ** 3).toFixed(2) + " GiB"
                : "Unavailable"}
              . Requires 13 GiB free for installation/recovery. Voice models are separate.
            </p>
            <p className="vm-hint">
              This validated compatibility runtime uses Python 3.10, which is end of support. A
              supported baseline must be qualified before general production redistribution.
            </p>
            <div className="vm-actions">
              <Button
                variant="primary"
                disabled={!status || active || ai.busy || status.install.installing}
                onClick={() => void ai.installRecommended()}
              >
                {status?.runtimeInstalled
                  ? status.runtimeVersion === status.recommendedRuntimeVersion
                    ? "Repair recommended runtime"
                    : "Update recommended runtime"
                  : "Install recommended runtime"}
              </Button>
              <Button disabled={!status || active || ai.busy} onClick={() => setRuntimeOpen(true)}>
                <FolderOpen size={15} />
                Install local package
              </Button>
              {status?.runtimeInstalled && (
                <Button disabled={active || ai.busy} onClick={() => void ai.rollback()}>
                  Rollback runtime
                </Button>
              )}
            </div>
            {status?.install.installing && (
              <div role="status" className="vm-hint">
                {status.download ? (
                  <>
                    <p>
                      {status.download.phase}: {status.download.component}
                    </p>
                    <progress
                      className="vm-download-progress"
                      aria-label="AI runtime download"
                      max={Math.max(1, status.download.totalBytes)}
                      value={Math.min(status.download.completedBytes, status.download.totalBytes)}
                    />
                    <p>
                      {(status.download.completedBytes / 1024 ** 3).toFixed(2)} /{" "}
                      {(status.download.totalBytes / 1024 ** 3).toFixed(2)} GiB ·{" "}
                      {status.download.completedComponents} / {status.download.totalComponents}{" "}
                      items
                    </p>
                  </>
                ) : (
                  <>
                    Verified files: {status.install.copiedFiles} / {status.install.totalFiles}
                  </>
                )}
                <Button onClick={() => void ai.cancelInstall()}>Cancel install</Button>
              </div>
            )}
            <p className="vm-hint">
              {status?.runtimeVersion ? `Installed: ${status.runtimeVersion}. ` : ""}
              Pinned official sources and SHA-256 verification. Retry resumes cached downloads; an
              interrupted update keeps your previous runtime. Downloads start only when requested.
            </p>
          </Card>
          <Card>
            <div className="vm-section-heading">
              <h2>Runtime telemetry</h2>
            </div>
            <dl className="vm-runtime-list">
              <div>
                <dt>Loaded provider</dt>
                <dd>{status?.provider ?? "Not loaded"}</dd>
              </div>
              <div>
                <dt>Model load</dt>
                <dd>{time(status?.loadingMs)}</dd>
              </div>
              <div>
                <dt>Conversion</dt>
                <dd>{time(status?.conversionMs)}</dd>
              </div>
              <div>
                <dt>IPC round trip</dt>
                <dd>{time(status?.roundTripMs)}</dd>
              </div>
              <div>
                <dt>Completed chunks</dt>
                <dd>{status?.completedChunks ?? "—"}</dd>
              </div>
              <div>
                <dt>Input / output queue</dt>
                <dd>
                  {status
                    ? `${status.queuedInputSamples} / ${status.queuedOutputSamples} samples`
                    : "Unavailable"}
                </dd>
              </div>
              <div>
                <dt>Dropped input / output</dt>
                <dd>
                  {status
                    ? `${status.droppedInputSamples} / ${status.droppedOutputSamples}`
                    : "Unavailable"}
                </dd>
              </div>
              <div>
                <dt>Missing output</dt>
                <dd>{status ? `${status.missingOutputSamples} samples` : "Unavailable"}</dd>
              </div>
              <div>
                <dt>Restarts</dt>
                <dd>{status?.restarts ?? "—"}</dd>
              </div>
            </dl>
            <p className="vm-hint">
              Conversion and IPC timings are measured by the active process. They are not full
              microphone-to-call latency. GPU/VRAM usage is not estimated.
            </p>
          </Card>
          <Card>
            <div className="vm-section-heading">
              <h2>
                <Cpu size={17} /> Hardware & dependencies
              </h2>
            </div>
            <Button
              disabled={!status?.runtimeInstalled || ai.busy || active}
              onClick={() => void ai.detectHardware()}
            >
              {ai.busy ? "Checking runtime…" : "Detect CPU / GPU"}
            </Button>
            {status?.hardware ? (
              <dl className="vm-runtime-list">
                <div>
                  <dt>CPU logical threads</dt>
                  <dd>{status.hardware.cpuThreads}</dd>
                </div>
                <div>
                  <dt>CUDA devices</dt>
                  <dd>{status.hardware.cudaDevices.join(", ") || "None detected"}</dd>
                </div>
                <div>
                  <dt>Available providers</dt>
                  <dd>{status.hardware.providers.join(", ")}</dd>
                </div>
                <div>
                  <dt>Python</dt>
                  <dd>{status.hardware.pythonVersion}</dd>
                </div>
                <div>
                  <dt>PyTorch</dt>
                  <dd>{status.hardware.torchVersion}</dd>
                </div>
                <div>
                  <dt>ONNX Runtime</dt>
                  <dd>{status.hardware.onnxRuntimeVersion}</dd>
                </div>
              </dl>
            ) : (
              <p className="vm-hint">
                Install a runtime, then check its actual CPU / GPU support. Detection does not
                benchmark a voice model.
              </p>
            )}
          </Card>
          <Card>
            <div className="vm-section-heading">
              <h2>
                <ShieldCheck size={17} />
                Private by design
              </h2>
            </div>
            <p className="vm-hint">
              AI receives PCM through private process pipes. No local web port, remote audio upload,
              automatic recording or voice-model download is opened.
            </p>
          </Card>
        </aside>
      </div>
      {(message || ai.error || status?.message || status?.install.message) && (
        <p role="status" className="vm-notice">
          {ai.error || status?.message || status?.install.message || message}
        </p>
      )}
      <StudioDialog
        open={officialOpen}
        onOpenChange={setOfficialOpen}
        title="Tsukuyomi-chan official voice"
      >
        <p className="vm-hint">
          CV. Rei Yumesaki. Download the original owner's 244 MiB archive, verify it, and convert
          Normal 1 to ONNX locally. This voice has custom terms, not an MIT model license.
        </p>
        <p className="vm-hint">
          Credit the original input voice and the Tsukuyomi-chan official RVC model when required.
          The owner restricts political/religious advocacy, attacks on people or organizations, and
          redistribution/use as reusable voice materials. Read the complete terms for your intended
          use.
        </p>
        <p>
          <a
            href="https://tyc.rei-yumesaki.net/work/software/rvc/terms/"
            target="_blank"
            rel="noreferrer"
          >
            Read the owner's complete model terms
          </a>
        </p>
        <label className="vm-check">
          <input
            type="checkbox"
            checked={officialRights}
            onChange={(e) => setOfficialRights(e.target.checked)}
          />{" "}
          I have read the owner’s terms and will use this voice within those rights.
        </label>
        <Button
          variant="primary"
          disabled={!officialRights || ai.busy}
          onClick={() => {
            setOfficialOpen(false);
            void ai.installOfficialVoice(officialRights).then(async (result) => {
              if (result) {
                setOfficialOpen(false);
                await refresh();
                setSelectedId("tsukuyomi-official-normal1");
              }
            });
          }}
        >
          Download & prepare official voice
        </Button>
      </StudioDialog>
      <StudioDialog
        open={importOpen}
        onOpenChange={setImportOpen}
        title="Import a licensed voice model"
      >
        <p className="vm-hint">
          Choose a model manifest and matching ONNX file. Checkpoint/pickle voice models and
          archives are unsupported.
        </p>
        <ToggleSwitch
          label="I have permission to use this voice and accept its model license"
          checked={rights}
          onChange={setRights}
        />
        <div className="vm-actions mt-5">
          <Button variant="primary" disabled={!rights || busy} onClick={() => void importModel()}>
            <FileUp size={15} />
            {busy ? "Importing…" : "Choose manifest & ONNX"}
          </Button>
        </div>
      </StudioDialog>
      <StudioDialog
        open={runtimeOpen}
        onOpenChange={setRuntimeOpen}
        title="Install a trusted runtime package"
      >
        <p className="vm-hint">
          Select an unpacked VoiceMorph runtime package from a publisher you trust. It includes
          executable Python, libraries and licensed embedding assets. The publisher's manifest hash
          must come from a trusted source.
        </p>
        <label className="vm-field">
          Package directory
          <input aria-label="Runtime package directory" readOnly value={runtimePath} />
        </label>
        <Button
          disabled={ai.busy}
          onClick={() =>
            void open({ directory: true, multiple: false })
              .then((path) => {
                if (typeof path === "string") setRuntimePath(path);
              })
              .catch((error) => setMessage(String(error)))
          }
        >
          <FolderOpen size={16} />
          Choose package
        </Button>
        <label className="vm-field mt-4">
          Publisher manifest SHA-256
          <input
            aria-label="Publisher manifest SHA-256"
            value={digest}
            onChange={(e) => setDigest(e.target.value)}
            maxLength={64}
            spellCheck={false}
          />
        </label>
        <ToggleSwitch
          label="I trust the runtime publisher and its executable contents"
          checked={trusted}
          onChange={setTrusted}
        />
        <div className="vm-actions mt-5">
          <Button
            variant="primary"
            disabled={!trusted || !runtimePath || !/^[a-f0-9]{64}$/i.test(digest.trim()) || ai.busy}
            onClick={() => void install()}
          >
            Verify & install
          </Button>
          {status?.install.installing && (
            <Button onClick={() => void ai.cancelInstall()}>Cancel installation</Button>
          )}
        </div>
      </StudioDialog>
    </section>
  );
}
