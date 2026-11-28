import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowLeft, AudioLines, BrainCircuit, Play, RefreshCw, Square } from "lucide-react";
import { VoicePackManager } from "./VoicePackManager";
import { packPhaseLabels, runningPackPhases } from "../types/ai";
import { AppHeader } from "./AppHeader";
import { Badge, Button, Card, Slider } from "./ui";
import { useAudioSession } from "../hooks/AudioSession";
import { useEffectParameters } from "../hooks/useEffectParameters";
import * as api from "../services/tauri";
import type {
  AgentVoice,
  AIModelStatus,
  AgentComparison,
  VoiceAnalysis,
  VoicePackStatus,
} from "../types/ai";
const originalProfiles: Record<string, { description: string; theme: string }> = {
  "wind-assassin": { description: "Bright, agile and confidently energetic.", theme: "wind" },
  "jade-guardian": { description: "Warm, composed and reassuringly clear.", theme: "jade" },
};
function message(e: unknown) {
  return e instanceof Error
    ? e.message
    : typeof e === "string"
      ? e
      : e && typeof e === "object" && "message" in e
        ? String(e.message)
        : "Operation failed. Check the model and runtime, then retry.";
}
export function AgentVoiceCollection({ onBack }: { onBack: () => void }) {
  const { ai, routing } = useAudioSession();
  const effects = useEffectParameters();
  const [voices, setVoices] = useState<AgentVoice[]>([]),
    [models, setModels] = useState<AIModelStatus[]>([]),
    [selected, setSelected] = useState("wind-assassin"),
    [busy, setBusy] = useState(""),
    [error, setError] = useState("");
  const [source, setSource] = useState(""),
    [sourceRights, setSourceRights] = useState(false),
    [comparison, setComparison] = useState<AgentComparison | null>(null),
    [analysis, setAnalysis] = useState<VoiceAnalysis | null>(null),
    [url, setUrl] = useState("");
  const request = useRef(0);
  const [engine, setEngine] = useState<"neural" | "hybrid">("neural");
  const [packOpen, setPackOpen] = useState(false);
  const [loadingVoiceId, setLoadingVoiceId] = useState<string | null>(null);
  const [packStatus, setPackStatus] = useState<VoicePackStatus | null>(null);
  const packPhaseRef = useRef("");
  const packRunning = !!packStatus && runningPackPhases.has(packStatus.phase);
  useEffect(() => {
    let live = true;
    const poll = async () => {
      try {
        const next = await api.getVoicePackStatus();
        if (!live) return;
        setPackStatus(next);
        if (next.phase === "ready" && packPhaseRef.current !== "ready") {
          await refresh();
        }
        packPhaseRef.current = next.phase;
      } catch {
        /* Native bridge errors are already reported by the session provider. */
      }
    };
    void poll();
    const timer = setInterval(() => void poll(), 500);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, []);
  const voice = voices.find((v) => v.id === selected);
  const visibleVoices = voices.filter((entry) =>
    Object.prototype.hasOwnProperty.call(originalProfiles, entry.id),
  );
  const active = !!ai.status && ["starting", "ready", "converting"].includes(ai.status.state);
  const runtimeFault = ai.status?.state === "faulted" ? ai.status.message : null;
  const loadingVoice = loadingVoiceId !== null || ai.status?.state === "starting";
  const loadingVoiceName = voices.find((entry) =>
    loadingVoiceId ? entry.id === loadingVoiceId : entry.binding?.modelId === ai.status?.modelId,
  )?.name;
  const displayedEngine = active ? (ai.status?.config.hybridDsp ? "hybrid" : "neural") : engine;
  async function activateValidated() {
    if (!ai.status) throw new Error("Desktop AI runtime status is unavailable");
    const saved = await ai.save({
      ...ai.status.config,
      hybridDsp: engine === "hybrid",
      failurePolicy: "mute",
    });
    if (!saved) throw new Error("Stop AI and save its configuration before activation");
    await api.startAgentVoice(selected);
    await ai.refresh();
  }
  function loadVoice(id: string, saveEngine: boolean) {
    if (busy) return;
    setLoadingVoiceId(id);
    void run("Loading neural model…", async () => {
      try {
        if (saveEngine) {
          await activateValidated();
        } else {
          await api.startAgentVoice(id);
          await ai.refresh();
        }
      } finally {
        setLoadingVoiceId(null);
      }
    });
  }
  async function refresh() {
    const [v, m] = await Promise.all([api.getAgentVoices(), api.getAIModels()]);
    setVoices(v);
    setModels(m);
  }
  useEffect(() => {
    let live = true;
    void Promise.all([api.getAgentVoices(), api.getAIModels()])
      .then(([v, m]) => {
        if (live) {
          setVoices(v);
          setModels(m);
        }
      })
      .catch((e) => {
        if (live) setError(message(e));
      });
    return () => {
      live = false;
    };
  }, []);
  useEffect(
    () => () => {
      if (url) URL.revokeObjectURL(url);
    },
    [url],
  );
  async function run(label: string, fn: () => Promise<void>) {
    if (busy) return;
    setBusy(label);
    setError("");
    try {
      await fn();
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy("");
    }
  }
  function choose(v: AgentVoice) {
    request.current++;
    setSelected(v.id);
    setComparison(null);
    setAnalysis(null);
    setUrl("");
  }
  async function play(kind: "original" | "dsp" | "neural" | "hybrid") {
    if (!comparison) return;
    const p = comparison[kind];
    setUrl(URL.createObjectURL(new Blob([new Uint8Array(p.wav)], { type: "audio/wav" })));
  }
  return (
    <section className="vm-page" aria-label="Agent Voices">
      <AppHeader
        title="Agent Voices"
        description="Independent neural identities. Your words, a different voice."
        action={
          <Button onClick={onBack}>
            <ArrowLeft size={16} />
            Voice Lab
          </Button>
        }
      />
      <p className="vm-hint">
        Two original RVC voice profiles. Import a trained pack to unlock local preview and
        activation.
      </p>
      {(error || runtimeFault || ai.error || effects.error) && (
        <p role="alert" className="vm-form-error">
          {error || runtimeFault || ai.error || effects.error}
        </p>
      )}
      {packOpen && (
        <VoicePackManager
          agentId={selected}
          models={models}
          onClose={() => setPackOpen(false)}
          onChanged={async () => {
            await refresh();
            await ai.refresh();
          }}
          onActivate={activateValidated}
        />
      )}
      <div className="vm-agent-layout">
        <Card>
          <div className="vm-section-heading">
            <h2>
              <BrainCircuit size={17} />
              Neural voice library
            </h2>
            <Button
              aria-label="Refresh Agent voices"
              disabled={!!busy}
              onClick={() => void run("Refreshing…", refresh)}
            >
              <RefreshCw size={16} />
            </Button>
          </div>
          <div className="vm-agent-grid vm-original-agent-grid">
            {visibleVoices.map((v) => (
              <article key={v.id} className="vm-agent-tile">
                <button
                  className={`vm-agent-choice ${v.id === selected ? "is-selected" : ""}`}
                  aria-pressed={v.id === selected}
                  onClick={() => choose(v)}
                  disabled={!!busy}
                >
                  <span
                    aria-hidden="true"
                    className={`vm-original-agent-art is-${originalProfiles[v.id].theme}`}
                  />
                  <strong>{v.name}</strong>
                  <span>{originalProfiles[v.id].description}</span>
                  <Badge
                    tone={
                      v.state === "ready" ? "success" : v.state === "error" ? "warning" : "neutral"
                    }
                  >
                    {loadingVoiceId === v.id ||
                    (v.binding &&
                      ai.status?.modelId === v.binding.modelId &&
                      ai.status.state === "starting")
                      ? "Loading"
                      : v.binding &&
                          ai.status?.modelId === v.binding.modelId &&
                          ai.status.state === "faulted"
                        ? "Error"
                        : packStatus?.agentId === v.id &&
                            (packRunning ||
                              [
                                "error",
                                "incompatible",
                                "runtimeRequired",
                                "licenseUnverified",
                              ].includes(packStatus.phase))
                          ? packPhaseLabels[packStatus.phase]
                          : v.state === "ready"
                            ? "Ready"
                            : v.state === "error"
                              ? "Error"
                              : v.state === "notInstalled"
                                ? "Missing Model"
                                : "Validation required"}
                  </Badge>
                  {packRunning &&
                    packStatus?.agentId === v.id &&
                    packStatus.totalBytes > 0 &&
                    ["downloading", "installingConverter"].includes(packStatus.phase) && (
                      <progress
                        aria-label={`Download progress for ${v.name}`}
                        value={packStatus.completedBytes}
                        max={packStatus.totalBytes}
                      />
                    )}
                </button>
                <div className="vm-agent-card-actions">
                  <Button
                    disabled={!!busy || packRunning || active}
                    aria-label={`Import model for ${v.name}`}
                    onClick={() => {
                      choose(v);
                      setPackOpen(true);
                    }}
                  >
                    {v.binding ? "Revalidate pack" : "Import Pack"}
                  </Button>
                  {v.state === "ready" && (
                    <>
                      <Button
                        disabled={!!busy || packRunning || active}
                        onClick={() => {
                          choose(v);
                          loadVoice(v.id, false);
                        }}
                      >
                        Activate Voice
                      </Button>
                      <Button
                        disabled={!!busy}
                        onClick={() => {
                          choose(v);
                          document.getElementById("agent-comparison-lab")?.scrollIntoView();
                        }}
                      >
                        Preview Voice
                      </Button>
                    </>
                  )}
                  {v.binding && (
                    <Button
                      variant="danger"
                      disabled={!!busy || active || packRunning}
                      onClick={() => {
                        if (
                          !v.binding ||
                          !confirm(
                            `Delete the installed model for ${v.name}? Original files and download cache are retained.`,
                          )
                        )
                          return;
                        void run("Deleting installed model...", async () => {
                          await api.removeAIModel(v.binding!.modelId);
                          await api.removeAgentVoice(v.id);
                          await refresh();
                        });
                      }}
                    >
                      Delete Model
                    </Button>
                  )}
                </div>
              </article>
            ))}
          </div>
        </Card>
        <aside className="vm-agent-controls">
          <Card>
            <h2>{voice?.name ?? "Choose an Agent"}</h2>
            <p className="vm-hint">{voice?.message}</p>
            <p className="vm-hint">
              Ready means local inference passed. Listen to authorized samples before evaluating
              voice quality.
            </p>
            <label className="vm-field">
              Conversion engine
              <select
                value={displayedEngine}
                disabled={active}
                onChange={(e) => setEngine(e.target.value as "neural" | "hybrid")}
              >
                <option value="neural">RVC neural</option>
                <option value="hybrid">Hybrid · RVC + optional effects</option>
                <option disabled>Advanced neural · not installed / not qualified</option>
              </select>
            </label>
            <div className="vm-actions">
              <Button
                variant="primary"
                disabled={
                  !!busy ||
                  active ||
                  !ai.status?.runtimeInstalled ||
                  !voice?.binding ||
                  voice.state !== "ready"
                }
                onClick={() => loadVoice(selected, true)}
              >
                <Play size={15} />
                Load neural voice
              </Button>
              <Button disabled={!active || !!busy} onClick={() => void ai.stop()}>
                <Square size={15} />
                Stop AI
              </Button>
              <Button disabled={!!busy} onClick={() => void ai.classic()}>
                Use Classic DSP
              </Button>
            </div>
            {loadingVoice && (
              <div className="vm-voice-loading" role="status">
                {loadingVoiceName && <strong>{loadingVoiceName}</strong>}
                <span>
                  {ai.status?.state === "starting"
                    ? (ai.status.message ?? "Loading neural model…")
                    : "Loading neural model…"}
                </span>
                <progress className="vm-progress-track" aria-label="Neural voice loading" />
              </div>
            )}
            <p>
              <a className="vm-hint" href="#agent-comparison-lab">
                Open voice quality lab
              </a>
            </p>
            {!ai.status?.runtimeInstalled && (
              <p className="vm-hint">
                Runtime required. Install the managed runtime in AI Voice Studio.
              </p>
            )}
            <p className="vm-hint">
              Active model:{" "}
              {models.find((entry) => entry.id === ai.status?.modelId)?.name ??
                ai.status?.modelId ??
                "None"}
              . Engine: {ai.status?.state ?? "Unavailable"} · inference{" "}
              {ai.status?.conversionMs?.toFixed(1) ?? "—"} ms · IPC{" "}
              {ai.status?.roundTripMs?.toFixed(1) ?? "—"} ms. These are not end-to-end call latency.
            </p>
            <p className="vm-hint">
              CPU:{" "}
              {active ? (ai.status?.resources?.cpuCorePercent?.toFixed(1) ?? "Measuring") : "?"}%
              (one core = 100%) ? RAM:{" "}
              {active && ai.status?.resources
                ? (ai.status.resources.rssBytes / 1024 ** 2).toFixed(0)
                : "?"}{" "}
              MiB ? Torch allocated VRAM:{" "}
              {active && ai.status?.resources?.cudaAllocatedBytes != null
                ? (ai.status.resources.cudaAllocatedBytes / 1024 ** 2).toFixed(0)
                : "?"}{" "}
              MiB. GPU utilization: unavailable in live telemetry.
            </p>
            <p className="vm-hint">
              Virtual microphone: {routing.status?.state ?? "Check routing in Settings"}. Use CABLE
              Output as Discord input after routing VoiceMorph into CABLE Input.
            </p>
          </Card>
          {displayedEngine === "hybrid" && (
            <Card>
              <h2>
                <AudioLines size={17} />
                Optional character effects
              </h2>
              <p className="vm-hint">
                Applied after the neural base. Pitch/formant presets do not create another Agent
                identity.
              </p>
              <Slider
                label="Radio effect"
                value={effects.parameters.radioMix}
                min={0}
                max={1}
                step={0.05}
                onChange={(v) => void effects.update({ ...effects.parameters, radioMix: v })}
              />
              <Slider
                label="Ring modulation"
                value={effects.parameters.ringModMix}
                min={0}
                max={1}
                step={0.05}
                onChange={(v) => void effects.update({ ...effects.parameters, ringModMix: v })}
              />
              <Slider
                label="De-essing"
                value={effects.parameters.deEssStrength ?? 0}
                min={0}
                max={1}
                step={0.05}
                onChange={(v) => void effects.update({ ...effects.parameters, deEssStrength: v })}
              />
              <Button
                onClick={() =>
                  void effects.update({
                    ...effects.parameters,
                    bypassed: !effects.parameters.bypassed,
                  })
                }
              >
                {effects.parameters.bypassed ? "Enable effects" : "Bypass effects"}
              </Button>
            </Card>
          )}
        </aside>
      </div>
      <Card className="mt-5" id="agent-comparison-lab">
        <div className="vm-section-heading">
          <div>
            <h2>A/B voice quality lab</h2>
            <p>
              Use the same authorized recording for every target. Maximum five seconds; no audio is
              uploaded.
            </p>
          </div>
          <Button
            disabled={!!busy}
            onClick={() =>
              void run("Choosing recording…", async () => {
                const path = await open({
                  multiple: false,
                  filters: [{ name: "Local consented WAV", extensions: ["wav"] }],
                });
                if (typeof path === "string") {
                  setSource(path);
                  setSourceRights(false);
                  setComparison(null);
                  setAnalysis(null);
                  setUrl("");
                }
              })
            }
          >
            Choose local WAV
          </Button>
        </div>
        <p className="vm-hint">
          {source ||
            "Select an original microphone recording from Recordings, or another authorized WAV. Neural comparison requires 48 kHz."}
        </p>
        <label className="vm-check">
          <input
            type="checkbox"
            checked={sourceRights}
            onChange={(e) => setSourceRights(e.target.checked)}
          />
          I have permission to process this recording locally for comparison and calibration.
        </label>
        <div className="vm-actions">
          <Button
            disabled={
              !!busy ||
              !source ||
              !sourceRights ||
              !voice?.binding ||
              active ||
              !ai.status?.runtimeInstalled
            }
            onClick={() =>
              void run("Rendering original / DSP / neural / hybrid…", async () => {
                const generation = ++request.current;
                const result = await api.compareAgentVoice(selected, source, sourceRights);
                if (generation === request.current) {
                  setComparison(result);
                  setAnalysis(result.inputAnalysis);
                  setUrl("");
                }
              })
            }
          >
            Render four-way comparison
          </Button>
          <Button
            disabled={!!busy || !source || !sourceRights || !voice?.binding}
            onClick={() =>
              void run("Analyzing local voice…", async () =>
                setAnalysis(await api.calibrateAgentVoice(selected, source, sourceRights)),
              )
            }
          >
            Calibrate input for this voice
          </Button>
        </div>
        {busy && !loadingVoice && (
          <div role="status">
            <p>{busy}</p>
            <progress aria-label="Agent voice operation progress" />
          </div>
        )}
        {analysis && (
          <dl className="vm-runtime-list">
            <div>
              <dt>Estimated input F0 / spread</dt>
              <dd>
                {analysis.estimatedF0Hz?.toFixed(1) ?? "Unvoiced / unavailable"} /{" "}
                {analysis.f0SpreadHz?.toFixed(1) ?? "—"} Hz
              </dd>
            </div>
            <div>
              <dt>Input RMS / clipping</dt>
              <dd>
                {analysis.rmsDbfs.toFixed(1)} dBFS / {(analysis.clippingRate * 100).toFixed(3)}%
              </dd>
            </div>
            <div>
              <dt>Quietest-frame estimate</dt>
              <dd>{analysis.quietFrameDbfs.toFixed(1)} dBFS (not an isolated noise measurement)</dd>
            </div>
            <div>
              <dt>Input gain recommendation</dt>
              <dd>{analysis.recommendedGainDb.toFixed(1)} dB · apply manually in Voice Lab</dd>
            </div>
          </dl>
        )}
        {comparison && (
          <>
            <div className="vm-actions">
              {(["original", "dsp", "neural", "hybrid"] as const).map((k) => (
                <Button key={k} onClick={() => void play(k)}>
                  {k === "dsp"
                    ? "DSP only"
                    : k === "neural"
                      ? "Neural AI"
                      : k === "hybrid"
                        ? "Hybrid AI + DSP"
                        : "Original"}
                </Button>
              ))}
            </div>
            {url && (
              <audio key={url} src={url} controls autoPlay aria-label="Voice comparison playback" />
            )}
            <dl className="vm-runtime-list">
              <div>
                <dt>Provider / compute real-time factor</dt>
                <dd>
                  {comparison.provider} / {comparison.realTimeFactor.toFixed(2)}×{" "}
                  {comparison.realTimeFactor >= 1 ? "— slower than real time" : ""}
                </dd>
              </div>
              <div>
                <dt>Inference total / worst chunk</dt>
                <dd>
                  {comparison.inferenceMs.toFixed(1)} / {comparison.worstChunkMs.toFixed(1)} ms
                </dd>
              </div>
              <div>
                <dt>Chunk deadline misses</dt>
                <dd>
                  {comparison.missedDeadlines} · budget {comparison.deadlineMs.toFixed(1)} ms
                </dd>
              </div>
              <div>
                <dt>Discarded warm-up</dt>
                <dd>{comparison.warmupMs.toFixed(1)} ms (excluded from real-time factor)</dd>
              </div>
              <div>
                <dt>Model initialization</dt>
                <dd>
                  {comparison.initializationMs.toFixed(1)} ms (excluded from real-time factor)
                </dd>
              </div>
            </dl>
            <p className="vm-hint">
              Listen for naturalness, clarity, identity and artifacts. Outputs keep neural
              startup/tail and are not sample-aligned. These measurements do not prove perceptual
              quality, speaker separation, or virtual microphone latency. Calibration retains
              statistics per model, not the recording.
            </p>
          </>
        )}
      </Card>
    </section>
  );
}
