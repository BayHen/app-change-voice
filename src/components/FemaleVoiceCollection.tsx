import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ArrowLeft,
  AudioLines,
  BrainCircuit,
  Download,
  Heart,
  History,
  Play,
  Search,
  Square,
  SlidersHorizontal,
} from "lucide-react";
import { AppHeader } from "./AppHeader";
import { Badge, Button, Card, SearchInput, Slider, StudioDialog } from "./ui";
import { useAudioSession } from "../hooks/AudioSession";
import { useEffectParameters } from "../hooks/useEffectParameters";
import { useNoiseSettings } from "../hooks/useNoiseSettings";
import { useVolumeSettings } from "../hooks/useVolumeSettings";
import * as api from "../services/tauri";
import type { VoicePresetSummary } from "../types/preset";
import type { AIModelStatus } from "../types/ai";
import type { EffectParameters } from "../types/audio";

const modelId = "tsukuyomi-official-normal1";
const preferenceKey = "voicemorph-female-library-v1";
type Preferences = { favorites: string[]; recent: string[] };
function readPreferences(): Preferences {
  try {
    const p = JSON.parse(localStorage.getItem(preferenceKey) ?? "{}");
    const ids = (v: unknown) =>
      Array.isArray(v)
        ? v.filter((id): id is string => typeof id === "string" && id.length < 100).slice(0, 40)
        : [];
    return { favorites: ids(p.favorites), recent: ids(p.recent) };
  } catch {
    return { favorites: [], recent: [] };
  }
}
export function FemaleVoiceCollection({ onBack }: { onBack: () => void }) {
  const { ai, runtime } = useAudioSession();
  const effects = useEffectParameters(),
    noise = useNoiseSettings(),
    volume = useVolumeSettings();
  const [presets, setPresets] = useState<VoicePresetSummary[]>([]),
    [models, setModels] = useState<AIModelStatus[]>([]);
  const [selectedId, setSelectedId] = useState(""),
    [query, setQuery] = useState("");
  const [activeDspId, setActiveDspId] = useState<string | null>(null);
  const [filter, setFilter] = useState("All"),
    [scope, setScope] = useState("All voices");
  const [preferences, setPreferences] = useState(readPreferences),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [loading, setLoading] = useState(true);
  const [source, setSource] = useState<string | null>(null),
    [sourceRights, setSourceRights] = useState(false),
    [preview, setPreview] = useState<api.VoicePreview | null>(null),
    [playing, setPlaying] = useState(false);
  const [downloadOpen, setDownloadOpen] = useState(false),
    [modelRights, setModelRights] = useState(false);
  const player = useRef<HTMLAudioElement | null>(null),
    blobUrl = useRef<string | null>(null);
  const previewGeneration = useRef(0);
  const selected = presets.find((p) => p.id === selectedId),
    isAI = selectedId === modelId;
  const model = models.find((m) => m.id === modelId),
    verified = !!model?.integrityVerified && !!model?.consentConfirmed;
  const liveAI = !!ai.status && ["starting", "ready", "converting"].includes(ai.status.state);
  const processingAI = ai.status?.mode === "wOkada";
  const activeAI =
    ai.status?.modelId === modelId && ["ready", "converting"].includes(ai.status.state);
  useEffect(() => {
    let mounted = true;
    Promise.all([api.getAllPresets(), api.getAIModels(), api.getActivePresetId()])
      .then(([p, m, active]) => {
        if (mounted) {
          const female = p.filter((p) => p.category === "Female");
          setPresets(female);
          setSelectedId(female[0]?.id ?? modelId);
          setModels(m);
          setActiveDspId(active);
        }
      })
      .catch((e) => {
        if (mounted) setError(String(e));
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });
    return () => {
      mounted = false;
      previewGeneration.current += 1;
      player.current?.pause();
      if (blobUrl.current) URL.revokeObjectURL(blobUrl.current);
    };
  }, []);
  function savePreferences(next: Preferences) {
    try {
      localStorage.setItem(preferenceKey, JSON.stringify(next));
      setPreferences(next);
    } catch (e) {
      setError(`Could not save library preferences: ${String(e)}`);
    }
  }
  function favorite(id: string) {
    savePreferences({
      ...preferences,
      favorites: preferences.favorites.includes(id)
        ? preferences.favorites.filter((v) => v !== id)
        : [...preferences.favorites, id],
    });
  }
  function remember(id: string) {
    savePreferences({
      ...preferences,
      recent: [id, ...preferences.recent.filter((v) => v !== id)].slice(0, 20),
    });
  }
  async function run(operation: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await operation();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  function select(id: string) {
    previewGeneration.current += 1;
    setSelectedId(id);
    setPreview(null);
    player.current?.pause();
    setPlaying(false);
  }
  async function apply() {
    await run(async () => {
      if (isAI) {
        const result = await ai.start(modelId);
        if (!result) throw new Error("AI could not start; review the engine error below.");
      } else if (selected) {
        await api.applyPreset(selected.id);
        await api.selectClassicVoice();
        await effects.refresh();
        await noise.commit({
          ...noise.settings,
          enabled: selected.parameters.noiseSuppression,
          gateEnabled: selected.parameters.noiseGate,
          limiterEnabled: selected.parameters.limiterEnabled,
          limiterCeilingDbfs: selected.parameters.limiterCeilingDbfs,
        });
        await volume.commit({
          ...volume.settings,
          inputPercent: selected.parameters.inputGainPercent,
          outputPercent: selected.parameters.outputGainPercent,
        });
        await ai.refresh();
        setActiveDspId(selected.id);
      }
      remember(selectedId);
    });
  }
  async function change(patch: Partial<EffectParameters>) {
    await run(async () => {
      // Editing an inactive card first applies that card, never another selected voice's values.
      if (!selected) return;
      if (selectedId !== activeDspId) {
        const applied = await api.applyPreset(selectedId);
        await api.selectClassicVoice();
        await effects.update({ ...applied.parameters, ...patch });
        await ai.refresh();
      } else await effects.update({ ...effects.parameters, ...patch });
      setActiveDspId(selectedId);
    });
  }
  async function chooseSource() {
    const path = await open({
      multiple: false,
      filters: [{ name: "Local preview WAV", extensions: ["wav"] }],
    });
    if (typeof path === "string") {
      setSource(path);
      setSourceRights(false);
      setPreview(null);
    }
  }
  async function playPreview() {
    await run(async () => {
      if (!source || !sourceRights) return;
      const generation = ++previewGeneration.current;
      player.current?.pause();
      setPlaying(false);
      const result = isAI
        ? await api.renderAIVoicePreview(source, true)
        : await api.renderVoicePreview(
            selectedId,
            source,
            true,
            selectedId === activeDspId ? effects.parameters : undefined,
          );
      if (generation !== previewGeneration.current) return;
      setPreview(result);
      if (blobUrl.current) URL.revokeObjectURL(blobUrl.current);
      blobUrl.current = URL.createObjectURL(
        new Blob([new Uint8Array(result.wav)], { type: "audio/wav" }),
      );
      const audio = new Audio(blobUrl.current);
      player.current = audio;
      audio.onended = () => setPlaying(false);
      await audio.play();
      if (generation !== previewGeneration.current) {
        audio.pause();
        return;
      }
      setPlaying(true);
    });
  }
  const values =
    selected && selectedId !== activeDspId
      ? {
          ...effects.parameters,
          pitchSemitones: selected.pitchSemitones,
          formantSemitones: selected.formantSemitones,
          bassDb: selected.parameters.bassDb,
          trebleDb: selected.parameters.trebleDb,
          voiceIntensity: selected.parameters.voiceIntensity ?? 1,
          deEssStrength: selected.parameters.deEssStrength ?? 0,
          compressionRatio: selected.parameters.compressionRatio ?? 1,
        }
      : effects.parameters;
  const name = isAI ? "Tsukuyomi-chan · Normal 1" : (selected?.name ?? "Choose a voice");
  const entries = [
    ...presets.map((p) => ({
      id: p.id,
      name: p.name,
      description: p.description,
      ai: false,
      preset: p,
    })),
    {
      id: modelId,
      name: "Tsukuyomi-chan · Normal 1",
      description:
        "Official female RVC model by Rei Yumesaki. One AI speaker identity; Japanese source voice.",
      ai: true,
      preset: null,
    },
  ].sort((a, b) => Number(b.ai) - Number(a.ai));
  let visible = entries.filter(
    (v) =>
      (filter === "All" || (filter === "AI" ? v.ai : !v.ai)) &&
      (scope !== "Favorites" || preferences.favorites.includes(v.id)) &&
      (scope !== "Recent" || preferences.recent.includes(v.id)) &&
      `${v.name} ${v.description}`.toLowerCase().includes(query.toLowerCase()),
  );
  if (scope === "Recent")
    visible = visible.sort(
      (a, b) => preferences.recent.indexOf(a.id) - preferences.recent.indexOf(b.id),
    );
  return (
    <div className="vm-page vm-female-library">
      <AppHeader
        title="Female Voice Collection"
        description="Find a tone that fits your voice. Refine it with your own speech."
        action={
          <Button onClick={onBack}>
            <ArrowLeft size={15} />
            Voice Lab
          </Button>
        }
      />
      <div className="vm-library-toolbar">
        <SearchInput label="Search female voices" value={query} onChange={setQuery} />
        <Badge>Female</Badge>
        <div className="vm-library-filters" role="group" aria-label="Voice engine filter">
          {["All", "AI", "Classic DSP"].map((value) => (
            <Button key={value} aria-pressed={filter === value} onClick={() => setFilter(value)}>
              {value}
            </Button>
          ))}
        </div>
        <label className="vm-field">
          Show
          <select
            aria-label="Library scope"
            value={scope}
            onChange={(e) => setScope(e.target.value)}
          >
            {["All voices", "Favorites", "Recent"].map((value) => (
              <option key={value}>{value}</option>
            ))}
          </select>
        </label>
      </div>
      {(error || ai.error || effects.error || noise.error || volume.error) && (
        <p role="alert" className="vm-form-error">
          {error || ai.error || effects.error || noise.error || volume.error}
        </p>
      )}
      <p className="vm-hint">
        AI changes speaker timbre with a licensed model. Classic DSP shapes your own voice. Regional
        style names do not translate speech or create an accent; Vietnamese intelligibility is not
        yet verified.
      </p>
      <div className="vm-library-layout">
        <section aria-label="Female voice library">
          <div className="vm-section-heading">
            <h2>{loading ? "Loading voices…" : `${visible.length} voices`}</h2>
            <span className="vm-hint">20 DSP starting points · 1 reviewed AI identity</span>
          </div>
          <div className="vm-female-cards">
            {visible.map((voice, index) => (
              <article
                key={voice.id}
                className={`vm-voice-card ${selectedId === voice.id ? "is-selected" : ""}`}
              >
                <div className="vm-voice-card-top">
                  <Badge>{voice.ai ? "AI · RVC" : "Classic DSP"}</Badge>
                  <button
                    type="button"
                    className="vm-icon-button"
                    aria-label={`Favorite ${voice.name}`}
                    aria-pressed={preferences.favorites.includes(voice.id)}
                    onClick={() => favorite(voice.id)}
                  >
                    <Heart
                      size={17}
                      fill={preferences.favorites.includes(voice.id) ? "currentColor" : "none"}
                    />
                  </button>
                </div>
                <button
                  type="button"
                  className="vm-voice-select"
                  onClick={() => select(voice.id)}
                  aria-pressed={selectedId === voice.id}
                  aria-label={`Select ${voice.name}`}
                >
                  <span className={`vm-voice-art voice-art-${index % 4}`} aria-hidden="true">
                    {voice.ai ? <BrainCircuit size={30} /> : <AudioLines size={30} />}
                    <span className="vm-voice-spectrum">
                      {[10, 23, 17, 32, 25, 14, 28, 19, 35, 16, 24, 12].map((height, i) => (
                        <i key={i} style={{ height }} />
                      ))}
                    </span>
                  </span>
                  <strong>{voice.name}</strong>
                  <span>{voice.description}</span>
                </button>
                <div className="vm-voice-card-footer">
                  <span>
                    {voice.ai
                      ? activeAI
                        ? "Model loaded"
                        : verified
                          ? "File verified · load to validate"
                          : "Download required"
                      : `Pitch ${voice.preset!.pitchSemitones > 0 ? "+" : ""}${voice.preset!.pitchSemitones} st · resonance ${voice.preset!.formantSemitones}`}
                  </span>
                  {preferences.recent.includes(voice.id) && (
                    <History size={13} aria-label="Recently used" />
                  )}
                </div>
              </article>
            ))}
          </div>
          {!loading && !visible.length && (
            <Card>
              <Search size={22} />
              <h3>No matching voices</h3>
              <p className="vm-hint">Try another search or show all voices.</p>
            </Card>
          )}
        </section>
        <aside className="vm-voice-inspector">
          <Card>
            <div className="vm-section-heading">
              <h2>
                <SlidersHorizontal size={16} />
                Voice settings
              </h2>
              <Badge>{isAI ? "AI" : "DSP"}</Badge>
            </div>
            <h3>{name}</h3>
            <p className="vm-hint">
              {isAI
                ? "Model-specific starting point. Source is Japanese; Vietnamese/male-input naturalness is unverified."
                : selected?.description}
            </p>
            <Button
              variant="primary"
              disabled={
                busy || loading || (isAI && (!verified || !ai.status?.runtimeInstalled || ai.busy))
              }
              onClick={() => void apply()}
            >
              {isAI ? (liveAI ? "Restart with this AI voice" : "Use AI voice") : "Use DSP voice"}
            </Button>
            {!isAI && (
              <p className="vm-hint">
                Select Use DSP voice to load its starting values. Controls below edit the live
                Classic DSP chain; noise and gains also affect live input/output.
              </p>
            )}
            {isAI ? (
              <>
                <p className="vm-hint">
                  {activeAI
                    ? "Model loaded successfully. Enable Monitor and routing explicitly to send your microphone."
                    : verified
                      ? "Model file and consent verified. Load it to validate runtime compatibility."
                      : "Model is not installed or did not pass integrity checks."}
                </p>
                <Button
                  disabled={busy || ai.busy || ai.status?.install.installing}
                  onClick={() => setDownloadOpen(true)}
                >
                  <Download size={15} />
                  {verified ? "Review model / reinstall" : "Download official model"}
                </Button>
                {!ai.status?.runtimeInstalled && (
                  <Button disabled={ai.busy || busy} onClick={() => void ai.installRecommended()}>
                    Install recommended AI runtime
                  </Button>
                )}
                {ai.status?.download && (
                  <div role="status">
                    <p>
                      {ai.status.download.phase}: {ai.status.download.component}
                    </p>
                    <progress
                      max={Math.max(1, ai.status.download.totalBytes)}
                      value={ai.status.download.completedBytes}
                    />
                    <p className="vm-hint">
                      {ai.status.download.completedBytes.toLocaleString()} /{" "}
                      {ai.status.download.totalBytes.toLocaleString()} bytes
                    </p>
                  </div>
                )}
                {ai.status?.install.installing && (
                  <p role="status">
                    Verifying / preparing {ai.status.install.copiedFiles} /{" "}
                    {ai.status.install.totalFiles} files
                  </p>
                )}
                <Slider
                  label="AI pitch transpose"
                  min={-12}
                  max={12}
                  step={0.5}
                  unit=" st"
                  value={ai.status?.config.pitchSemitones ?? 0}
                  disabled={busy || liveAI || !ai.status}
                  onChange={(value) => {
                    if (ai.status) void ai.save({ ...ai.status.config, pitchSemitones: value });
                  }}
                />
                <label className="vm-field">
                  AI quality profile
                  <select
                    aria-label="Female AI quality profile"
                    disabled={!ai.status || liveAI || ai.busy}
                    value={ai.status?.config.chunkFrames ?? 8192}
                    onChange={(e) => {
                      if (ai.status)
                        void ai.save({
                          ...ai.status.config,
                          chunkFrames: Number(e.target.value),
                          crossfadeFrames: 1024,
                          extraContextFrames: 4096,
                        });
                    }}
                  >
                    <option value={2048} disabled>
                      2,048 · not qualified
                    </option>
                    <option value={4096}>Low Latency · 4,096 (after warmup)</option>
                    <option value={8192}>Balanced · 8,192</option>
                    <option value={16384}>High Quality · 16,384</option>
                  </select>
                </label>
                <p className="vm-hint">
                  Measured on RTX 3080 / this model. CPU did not sustain realtime. Stop AI before
                  changing settings. AI formant, intensity and post-DSP dynamics are unsupported
                  here.
                </p>
                <Button disabled={ai.busy || !processingAI} onClick={() => void ai.stop()}>
                  <Square size={14} />
                  Stop AI (mute)
                </Button>
                <p className="vm-hint">
                  Credit: Tsukuyomi-chan official RVC model (CV. Rei Yumesaki). Credit your source
                  voice too.{" "}
                  <a
                    href="https://tyc.rei-yumesaki.net/work/software/rvc/terms/"
                    target="_blank"
                    rel="noreferrer"
                  >
                    Read model terms
                  </a>
                  .
                </p>
              </>
            ) : (
              <>
                <Slider
                  label="Female pitch"
                  min={-12}
                  max={12}
                  step={0.1}
                  unit=" st"
                  value={values.pitchSemitones}
                  disabled={busy || effects.loading || processingAI}
                  onChange={(v) => void change({ pitchSemitones: v })}
                />
                <Slider
                  label="Female formant"
                  min={-6}
                  max={6}
                  step={0.1}
                  unit=" st"
                  value={values.formantSemitones}
                  disabled={busy || effects.loading || processingAI}
                  onChange={(v) => void change({ formantSemitones: v })}
                />
                <Slider
                  label="Voice intensity"
                  min={0}
                  max={1}
                  step={0.05}
                  value={values.voiceIntensity ?? 1}
                  disabled={busy || effects.loading || processingAI}
                  onChange={(v) => void change({ voiceIntensity: v })}
                />
                <Slider
                  label="Warmth · bass shelf"
                  min={-6}
                  max={6}
                  step={0.25}
                  unit=" dB"
                  value={values.bassDb}
                  disabled={busy || effects.loading || processingAI}
                  onChange={(v) => void change({ bassDb: v })}
                />
                <Slider
                  label="Brightness · treble shelf"
                  min={-6}
                  max={6}
                  step={0.25}
                  unit=" dB"
                  value={values.trebleDb}
                  disabled={busy || effects.loading || processingAI}
                  onChange={(v) => void change({ trebleDb: v })}
                />
                <details>
                  <summary>Advanced vocal dynamics</summary>
                  <Slider
                    label="De-essing strength"
                    min={0}
                    max={1}
                    step={0.05}
                    value={values.deEssStrength ?? 0}
                    disabled={busy || effects.loading || processingAI}
                    onChange={(v) => void change({ deEssStrength: v })}
                  />
                  <Slider
                    label="Compression ratio"
                    min={1}
                    max={4}
                    step={0.1}
                    unit=":1"
                    value={values.compressionRatio ?? 1}
                    disabled={busy || effects.loading || processingAI}
                    onChange={(v) => void change({ compressionRatio: v })}
                  />
                  <p className="vm-hint">
                    Split-band sibilant reduction; RMS compressor at −18 dBFS, no automatic makeup.
                    Final limiter protects the device output.
                  </p>
                </details>
              </>
            )}
            <details>
              <summary>Audio levels and noise</summary>
              <Slider
                label="Collection input gain"
                min={0}
                max={100}
                unit="%"
                value={volume.settings.inputPercent}
                disabled={volume.loading || busy}
                onChange={(v) => void volume.commit({ ...volume.settings, inputPercent: v })}
              />
              <Slider
                label="Collection output gain"
                min={0}
                max={100}
                unit="%"
                value={volume.settings.outputPercent}
                disabled={volume.loading || busy}
                onChange={(v) => void volume.commit({ ...volume.settings, outputPercent: v })}
              />
              <label className="vm-check">
                <input
                  type="checkbox"
                  checked={noise.settings.enabled}
                  disabled={noise.loading || busy}
                  onChange={(e) =>
                    void noise.commit({
                      ...noise.settings,
                      enabled: e.target.checked,
                      bypassed: false,
                    })
                  }
                />
                Noise reduction
              </label>
              <p className="vm-hint">
                Output gain controls headphone monitoring. Virtual routing uses its own final
                limiter.
              </p>
            </details>
          </Card>
          <Card>
            <h3>Listen with your sample</h3>
            <p className="vm-hint">
              Choose a WAV you have permission to use. Preview processes up to five seconds locally,
              plays on the system output and never starts your microphone or changes the live voice.
              AI preview needs 48 kHz and live AI stopped.
            </p>
            <Button disabled={busy} onClick={() => void run(chooseSource)}>
              Choose preview WAV
            </Button>
            <p className="vm-hint vm-preview-source">{source ?? "No sample selected"}</p>
            <label className="vm-check">
              <input
                type="checkbox"
                disabled={!source || busy}
                checked={sourceRights}
                onChange={(e) => setSourceRights(e.target.checked)}
              />
              I have permission to use this sample.
            </label>
            <div className="vm-actions">
              <Button
                disabled={
                  !source ||
                  !sourceRights ||
                  busy ||
                  !selectedId ||
                  (isAI && (!verified || liveAI || !ai.status?.runtimeInstalled))
                }
                onClick={() => void playPreview()}
              >
                <Play size={14} />
                {busy ? "Working…" : "Play voice preview"}
              </Button>
              <Button
                disabled={!playing}
                onClick={() => {
                  player.current?.pause();
                  setPlaying(false);
                }}
              >
                <Square size={14} />
                Stop
              </Button>
            </div>
            {preview && (
              <p role="status" className="vm-hint">
                Local preview: {preview.sampleRate.toLocaleString()} Hz ·{" "}
                {(preview.frames / preview.sampleRate).toFixed(2)} s · peak{" "}
                {preview.peak.toFixed(3)} · RMS {preview.rms.toFixed(3)} · full-scale samples{" "}
                {preview.fullScaleSamples}. Processing {preview.processingMs.toFixed(1)} ms
                (offline, not live latency).
              </p>
            )}
            <p className="vm-hint">
              {runtime.status.state === "running"
                ? "Live engine continues independently; wear headphones during preview."
                : "Microphone is stopped."}
            </p>
          </Card>
        </aside>
      </div>
      <StudioDialog
        open={downloadOpen}
        onOpenChange={setDownloadOpen}
        title="Official female AI voice"
      >
        <p>
          Tsukuyomi-chan Normal 1 uses custom attribution and usage terms from the original voice
          owner. It is one speaker model, not a pack of twenty AI identities.
        </p>
        <p>
          <a
            href="https://tyc.rei-yumesaki.net/work/software/rvc/terms/"
            target="_blank"
            rel="noreferrer"
          >
            Review voice and redistribution terms
          </a>
        </p>
        <label className="vm-check">
          <input
            type="checkbox"
            checked={modelRights}
            onChange={(e) => setModelRights(e.target.checked)}
          />
          I accept the original-owner terms and will credit the model and source voice.
        </label>
        <Button
          variant="primary"
          disabled={!modelRights || ai.busy || !ai.status?.runtimeInstalled}
          onClick={() =>
            void run(async () => {
              const result = await ai.installOfficialVoice(modelRights);
              if (!result) throw new Error("Model download failed; review AI engine status.");
              setModels(await api.getAIModels());
              setDownloadOpen(false);
            })
          }
        >
          Download and prepare model
        </Button>
      </StudioDialog>
    </div>
  );
}
