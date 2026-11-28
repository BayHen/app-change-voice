import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Download, FolderOpen, ShieldCheck, X } from "lucide-react";
import { Badge, Button, StudioDialog } from "./ui";
import * as api from "../services/tauri";
import type { AIModelStatus, VoicePackRemote, VoicePackStatus } from "../types/ai";
import { packPhaseLabels, runningPackPhases } from "../types/ai";
const message = (e: unknown) =>
  e instanceof Error
    ? e.message
    : typeof e === "string"
      ? e
      : e && typeof e === "object" && "message" in e
        ? String(e.message)
        : "Voice-pack operation failed. Retry or inspect the model source.";
export function VoicePackManager({
  agentId,
  models,
  onClose,
  onChanged,
  onActivate,
  preferRemote = false,
  initialExistingId,
  onRepositoryInspected,
}: {
  agentId: string;
  models: AIModelStatus[];
  onClose: () => void;
  onChanged: () => Promise<void>;
  onActivate: () => Promise<void>;
  preferRemote?: boolean;
  initialExistingId?: string;
  onRepositoryInspected?: (artifact: VoicePackRemote) => void;
}) {
  const original = agentId === "wind-assassin" || agentId === "jade-guardian";
  const displayName =
    agentId === "wind-assassin"
      ? "Wind Assassin"
      : agentId === "jade-guardian"
        ? "Jade Guardian"
        : agentId;
  const candidate = ["jett", "sage"].includes(agentId)
    ? `https://huggingface.co/fox3000foxy/valorant_rvc/blob/main/${agentId}/${agentId}.pth`
    : "";
  const [mode, setMode] = useState<"local" | "remote" | "existing">(
    initialExistingId !== undefined ? "existing" : preferRemote && candidate ? "remote" : "local",
  );
  const [url, setUrl] = useState(candidate),
    [path, setPath] = useState(""),
    [existing, setExisting] = useState(initialExistingId ?? ""),
    [remote, setRemote] = useState<VoicePackRemote | null>(null),
    [source, setSource] = useState(""),
    [license, setLicense] = useState(""),
    [attribution, setAttribution] = useState(""),
    [rights, setRights] = useState(false),
    [status, setStatus] = useState<VoicePackStatus | null>(null),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const started = useRef(false),
    mounted = useRef(true);
  const running = !!status && runningPackPhases.has(status.phase);
  const changed = useRef(onChanged);
  useEffect(() => {
    changed.current = onChanged;
  });
  useEffect(() => {
    mounted.current = true;
    const poll = async () => {
      try {
        const result = await api.getVoicePackStatus();
        if (!mounted.current) return;
        setStatus(result);
        if (
          started.current &&
          [
            "ready",
            "error",
            "incompatible",
            "licenseUnverified",
            "cancelled",
            "runtimeRequired",
          ].includes(result.phase)
        ) {
          started.current = false;
          await changed.current();
        }
      } catch (e) {
        if (mounted.current) setError(message(e));
      }
    };
    void poll();
    const timer = setInterval(() => void poll(), 500);
    return () => {
      mounted.current = false;
      clearInterval(timer);
    };
  }, []);
  async function inspect() {
    setBusy(true);
    setError("");
    try {
      const artifact = await api.inspectHFVoice(url);
      setRemote(artifact);
      onRepositoryInspected?.(artifact);
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }
  async function install() {
    setBusy(true);
    setError("");
    try {
      const result = await api.installVoicePack({
        agentId,
        localPath: mode === "local" ? path : null,
        hfUrl: mode === "remote" ? url : null,
        existingModelId: mode === "existing" ? existing : null,
        indexPath: null,
        identityName: null,
        sourceUrl: source || null,
        licenseUrl: license || null,
        attribution: attribution || null,
        rightsConfirmed: rights,
      });
      started.current = true;
      setStatus(result);
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <StudioDialog
      open
      title={`Voice Pack Manager · ${displayName}`}
      onOpenChange={(value) => {
        if (!value && !running && !busy) onClose();
      }}
    >
      <p className="vm-hint">
        A pack becomes Ready only after real neural inference succeeds. Model weights are not
        included with the Agent slots.
      </p>
      {!original && (
        <div role="group" aria-label="Voice pack source" className="vm-actions">
          {(
            [
              ["local", "Local PTH / ONNX"],
              ["remote", "Hugging Face URL"],
              ["existing", "Installed model"],
            ] as const
          ).map(([value, label]) => (
            <Button
              key={value}
              variant={mode === value ? "primary" : undefined}
              disabled={running || busy}
              aria-pressed={mode === value}
              onClick={() => {
                setMode(value);
                setRights(false);
                setRemote(null);
              }}
            >
              {label}
            </Button>
          ))}
        </div>
      )}
      {mode === "local" && (
        <>
          <Button
            disabled={running || busy}
            onClick={() =>
              void open({
                multiple: false,
                filters: original
                  ? [{ name: "Original RVC voice pack", extensions: ["json"] }]
                  : [{ name: "RVC model", extensions: ["pth", "onnx"] }],
              })
                .then((file) => {
                  if (typeof file === "string") {
                    setPath(file);
                    setRights(false);
                  }
                })
                .catch((e) => setError(message(e)))
            }
          >
            <FolderOpen size={16} />
            {original ? "Choose voice-pack.json" : "Choose PTH or ONNX"}
          </Button>
          <p className="vm-hint vm-pack-path">
            {path ||
              (original
                ? "Choose the exported pack manifest beside its checkpoint, ONNX and feature index."
                : "Original file stays unchanged. PTH is loaded with restricted deserialization in an isolated worker.")}
          </p>
        </>
      )}
      {mode === "remote" && (
        <>
          <label className="vm-field">
            Hugging Face model file URL
            <input
              value={url}
              disabled={running || busy}
              onChange={(e) => {
                setUrl(e.target.value);
                setRemote(null);
                setRights(false);
              }}
            />
          </label>
          <Button disabled={running || busy || !url} onClick={() => void inspect()}>
            Check repository metadata
          </Button>
          {remote && (
            <div className="vm-notice">
              <Badge tone={remote.state === "licenseUnverified" ? "warning" : "neutral"}>
                {remote.state === "licenseUnverified"
                  ? "License Unverified"
                  : "Repository license declared · voice rights still required"}
              </Badge>
              <p>
                {remote.filename} · {(remote.bytes / 1024 ** 2).toFixed(1)} MiB
              </p>
              <p className="vm-hint vm-pack-path">
                Revision: {remote.revision}
                <br />
                SHA-256: {remote.sha256}
                <br />
                Declared license: {remote.declaredLicense ?? "None"}
              </p>
              {remote.state === "licenseUnverified" && (
                <p>
                  Download is blocked. Public availability does not establish model distribution or
                  actor voice rights. Import an authorized original voice instead.
                </p>
              )}
            </div>
          )}
        </>
      )}
      {mode === "existing" && (
        <label className="vm-field">
          Installed neural model
          <select
            value={existing}
            disabled={running || busy}
            onChange={(e) => setExisting(e.target.value)}
          >
            <option value="">Select a model to validate and assign</option>
            {models
              .filter(
                (m) => m.backend === "w-okada-rvc" && m.consentConfirmed && m.integrityVerified,
              )
              .map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
          </select>
        </label>
      )}
      {!original &&
        (mode === "local" || (mode === "remote" && remote?.state === "declaredLicense")) && (
          <>
            <label className="vm-field">
              Model source / owner URL
              <input
                type="url"
                value={source}
                disabled={running}
                onChange={(e) => setSource(e.target.value)}
                placeholder="https://…"
              />
            </label>
            <label className="vm-field">
              Model license and voice-permission URL
              <input
                type="url"
                value={license}
                disabled={running}
                onChange={(e) => setLicense(e.target.value)}
                placeholder="https://…"
              />
            </label>
            <label className="vm-field">
              Voice owner attribution
              <input
                value={attribution}
                disabled={running}
                onChange={(e) => setAttribution(e.target.value)}
              />
            </label>
            <p className="vm-hint">
              Known official Tsukuyomi checksums populate owner metadata automatically. Other
              imports retain your rights declaration; repository metadata is not actor permission.
            </p>
            <label className="vm-check">
              <input
                type="checkbox"
                disabled={running}
                checked={rights}
                onChange={(e) => setRights(e.target.checked)}
              />
              I have model and voice authorization for this use and accept the complete terms.
            </label>
          </>
        )}
      {original && (
        <label className="vm-check">
          <input
            type="checkbox"
            checked={rights}
            disabled={running || busy}
            onChange={(event) => setRights(event.target.checked)}
          />
          I have permission to use this model and its original voice.
        </label>
      )}
      {error && (
        <p role="alert" className="vm-form-error">
          {error}
        </p>
      )}
      {status && status.agentId === agentId && (
        <div role="status" className="vm-notice">
          <strong>{packPhaseLabels[status.phase] ?? status.phase}</strong>
          <p>{status.message}</p>
          {running &&
            (["downloading", "installingConverter"].includes(status.phase) ? (
              <>
                <progress
                  aria-label="Voice pack download progress"
                  max={Math.max(1, status.totalBytes)}
                  value={status.completedBytes}
                />
                <p>
                  {(status.completedBytes / 1024 ** 2).toFixed(1)} /{" "}
                  {(status.totalBytes / 1024 ** 2).toFixed(1)} MiB
                </p>
              </>
            ) : (
              <progress aria-label="Voice pack processing progress" />
            ))}
          {status.detected && (
            <details>
              <summary>Detected model details</summary>
              <pre className="vm-hint vm-pack-metadata">
                {JSON.stringify(status.detected, null, 2)}
              </pre>
            </details>
          )}
          {status.validation && (
            <p>
              Validated: {status.validation.provider} · {status.validation.chunks} real chunks · RTF{" "}
              {status.validation.realTimeFactor.toFixed(2)}
              {status.validation.realTimeFactor >= 1 ? " — slower than real time" : ""}. This is a
              synthetic inference smoke test, not listening validation or call latency.
            </p>
          )}
        </div>
      )}
      {status && running && status.agentId !== agentId && (
        <p role="status">A voice-pack job is running for {status.agentId}. Wait or cancel it.</p>
      )}
      <div className="vm-actions mt-5">
        <Button
          variant="primary"
          disabled={
            running ||
            busy ||
            (mode === "existing" ? !existing : !rights) ||
            (mode === "local" && !path) ||
            (mode === "remote" && (!remote || remote.state === "licenseUnverified"))
          }
          onClick={() => void install()}
        >
          <Download size={16} />
          {mode === "existing" ? "Validate & assign" : "Install, validate & assign"}
        </Button>
        {running && (
          <Button
            disabled={status?.phase === "assigning"}
            onClick={() => void api.cancelVoicePack().catch((e) => setError(message(e)))}
          >
            <X size={16} />
            Cancel · keep resumable cache
          </Button>
        )}
        {status?.phase === "ready" &&
          status.agentId === agentId &&
          models.some((model) => model.id === status.modelId) && (
            <>
              <Button
                variant="primary"
                disabled={busy}
                onClick={() => {
                  setBusy(true);
                  void onActivate()
                    .then(onClose)
                    .catch((e) => setError(message(e)))
                    .finally(() => setBusy(false));
                }}
              >
                <ShieldCheck size={16} />
                Activate validated voice
              </Button>
              <Button
                disabled={busy}
                onClick={() =>
                  void api
                    .resetAgentRecommendations(agentId)
                    .then(() => changed.current())
                    .catch((e) => setError(message(e)))
                }
              >
                Reset to recommended settings
              </Button>
            </>
          )}
      </div>
    </StudioDialog>
  );
}
