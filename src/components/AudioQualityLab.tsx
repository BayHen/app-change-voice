import { useEffect, useState } from "react";
import { useAudioSession } from "../hooks/AudioSession";
import { useVolumeSettings } from "../hooks/useVolumeSettings";
import { useNoiseSettings } from "../hooks/useNoiseSettings";
import { useEffectParameters } from "../hooks/useEffectParameters";
import { Card, Button, Badge, Slider } from "./ui";
import { ToggleSwitch } from "./ToggleSwitch";
import * as api from "../services/tauri";
import type { AudioCaptureStage, AudioQualitySettings, StageCaptureStatus } from "../types/audio";

const stages: Array<[AudioCaptureStage, string]> = [
  ["rawInput", "Raw input (all channels)"],
  ["bypassReference", "Dry bypass reference"],
  ["inputGain", "After input gain"],
  ["classicDsp", "Classic DSP"],
  ["aiInference", "AI inference · 48 kHz"],
  ["pcmBridge", "PCM bridge · native rate"],
  ["mixer", "Virtual mixer before limiter"],
  ["monitor", "Monitor after limiter"],
  ["virtualOutput", "Virtual output after limiter"],
];
const number = (value: number | null | undefined, unit = "", digits = 2) =>
  value == null ? "Unavailable" : `${value.toFixed(digits)}${unit}`;

export function AudioQualityLab() {
  const { runtime, ai, routing } = useAudioSession();
  const volume = useVolumeSettings(),
    noise = useNoiseSettings(),
    effects = useEffectParameters();
  const [quality, setQuality] = useState<AudioQualitySettings | null>(null),
    [capture, setCapture] = useState<StageCaptureStatus | null>(null);
  const [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [consent, setConsent] = useState(false);
  const [selected, setSelected] = useState<AudioCaptureStage[]>(["mixer", "virtualOutput"]),
    [seconds, setSeconds] = useState(5);
  const live = runtime.status.state === "running",
    metrics = runtime.status.metrics;
  useEffect(() => {
    let mounted = true;
    void Promise.all([api.getAudioQuality(), api.getStageCaptureStatus()])
      .then(([q, c]) => {
        if (mounted) {
          setQuality(q);
          setCapture(c);
        }
      })
      .catch((e) => {
        if (mounted) setError(String(e));
      });
    const timer = setInterval(
      () =>
        void api
          .getStageCaptureStatus()
          .then((c) => {
            if (mounted) setCapture(c);
          })
          .catch((e) => {
            if (mounted) setError(String(e));
          }),
      500,
    );
    return () => {
      mounted = false;
      clearInterval(timer);
    };
  }, []);
  async function run(operation: () => Promise<unknown>) {
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
  async function reset() {
    await runtime.stop();
    await volume.commit({
      inputPercent: 100,
      outputPercent: 100,
      inputMuted: false,
      outputMuted: false,
    });
    await noise.commit({
      enabled: false,
      bypassed: false,
      suppressionStrength: 0.5,
      gateEnabled: false,
      gateThresholdDbfs: -52,
      limiterEnabled: true,
      limiterCeilingDbfs: -1,
    });
    await effects.reset();
    setQuality(await api.setAudioQuality({ profile: "balanced" }));
  }
  const gpu = ai.status?.hardware
    ? ai.status.hardware.torchCudaAvailable && ai.status.config.provider !== "cpu"
    : null;
  const recommendedChunk =
    quality?.profile === "lowLatency" ? 4096 : quality?.profile === "highQuality" ? 16384 : 8192;
  const aiActive = ai.status && ["starting", "ready", "converting"].includes(ai.status.state);
  return (
    <section className="vm-page" aria-label="Audio Quality Lab">
      <div className="vm-section-heading">
        <div>
          <h2>Audio Quality Lab</h2>
          <p className="vm-hint">
            Inspect the real signal path. Diagnostic WAV recording is always opt-in and local.
          </p>
        </div>
        <Badge tone={live ? "success" : "neutral"}>{runtime.status.state}</Badge>
      </div>
      {(error || volume.error || noise.error || effects.error || ai.error) && (
        <p role="alert" className="vm-notice">
          {error || volume.error || noise.error || effects.error || ai.error}
        </p>
      )}
      <div className="vm-quality-grid">
        <Card>
          <h3>Quality and buffering</h3>
          <label className="vm-field">
            Audio quality profile
            <select
              aria-label="Audio quality profile"
              value={quality?.profile ?? "balanced"}
              disabled={!quality || busy}
              onChange={(e) =>
                setQuality({ profile: e.target.value as AudioQualitySettings["profile"] })
              }
            >
              <option value="lowLatency">Low Latency</option>
              <option value="balanced">Balanced · recommended</option>
              <option value="highQuality">High Quality</option>
            </select>
          </label>
          <p className="vm-hint">
            Device-supported buffer sizes are negotiated, not assumed. Profiles change startup
            headroom, requested buffers and sinc filter quality. Applying stops Monitor; restart it
            and routing explicitly.
          </p>
          <Button
            disabled={!quality || busy || capture?.running}
            onClick={() =>
              void run(async () => {
                setQuality(await api.setAudioQuality(quality!));
              })
            }
          >
            Apply audio quality
          </Button>
          <p className="vm-hint">
            {gpu === null
              ? "Detect hardware in AI Voice Studio before applying an AI chunk recommendation."
              : gpu
                ? `AI starting recommendation: ${recommendedChunk} frames with CUDA. Stop AI before changing chunk configuration. Verify the actual inference budget for your model.`
                : "CPU did not sustain realtime in the current model benchmark. Use Classic DSP/CUDA; CPU settings remain available manually in AI Voice Studio."}
          </p>
          <Button
            disabled={!quality || !ai.status || !gpu || !!aiActive || busy}
            onClick={() =>
              void run(async () => {
                await ai.save({
                  ...ai.status!.config,
                  chunkFrames: recommendedChunk,
                  crossfadeFrames: 1024,
                  extraContextFrames: 4096,
                });
              })
            }
          >
            Apply AI chunk recommendation
          </Button>
        </Card>
        <Card>
          <h3>Levels and processing</h3>
          {volume.loading ||
          noise.loading ||
          effects.loading ||
          volume.error ||
          noise.error ||
          effects.error ? (
            <p className="vm-hint">Backend controls loading or unavailable.</p>
          ) : (
            <>
              <Slider
                label="Quality lab input gain"
                min={0}
                max={100}
                value={volume.settings.inputPercent}
                unit="%"
                onChange={(v) => void volume.commit({ ...volume.settings, inputPercent: v })}
              />
              <Slider
                label="Quality lab output gain"
                min={0}
                max={100}
                value={volume.settings.outputPercent}
                unit="%"
                onChange={(v) => void volume.commit({ ...volume.settings, outputPercent: v })}
              />
              <ToggleSwitch
                label="Quality lab noise reduction"
                checked={noise.settings.enabled && !noise.settings.bypassed}
                onChange={(v) =>
                  void noise.commit({ ...noise.settings, enabled: v, bypassed: false })
                }
              />
              <ToggleSwitch
                label="Quality lab safety limiter"
                checked={noise.settings.limiterEnabled}
                onChange={(v) => void noise.commit({ ...noise.settings, limiterEnabled: v })}
              />
              <ToggleSwitch
                label="Classic DSP bypass"
                checked={effects.parameters.bypassed || !effects.parameters.enabled}
                onChange={(v) =>
                  void effects.update({ ...effects.parameters, bypassed: v, enabled: true })
                }
              />
            </>
          )}
          <p className="vm-hint">
            Noise processing is independent of Classic DSP bypass. Input gain precedes noise/DSP/AI.
            Output gain controls headphone playback; virtual routing has its own final limiter.
          </p>
          <Button disabled={ai.busy} onClick={() => void ai.classic()}>
            Use Classic DSP (AI bypass)
          </Button>
          <p className="vm-hint">
            Explicitly restores your Classic voice on active outputs. Stop AI alone keeps the AI
            voice muted.
          </p>
          <Button disabled={busy || capture?.running} onClick={() => void run(reset)}>
            Reset audio levels / DSP / noise
          </Button>
          <p className="vm-hint">
            Reset stops audio, selects Balanced and leaves the AI voice mode unchanged.
          </p>
        </Card>
        <Card>
          <h3>Measured telemetry</h3>
          <dl className="vm-quality-metrics">
            <dt>Native sample rate / channels</dt>
            <dd>
              {runtime.status.sampleRate == null
                ? "Unavailable"
                : `${runtime.status.sampleRate} Hz · ${runtime.status.inputChannels} in / ${runtime.status.outputChannels} out`}
            </dd>
            <dt>Device sample formats</dt>
            <dd>
              {runtime.status.inputSampleFormat ?? "Unavailable"} /{" "}
              {runtime.status.outputSampleFormat ?? "Unavailable"}
            </dd>
            <dt>Negotiated buffers</dt>
            <dd>
              {metrics
                ? `${metrics.inputBufferFrames} / ${metrics.outputBufferFrames} frames`
                : "Unavailable"}
            </dd>
            <dt>Input peak / RMS</dt>
            <dd>
              {number(metrics?.inputPeakDbfs, " dBFS")} / {number(metrics?.inputRmsDbfs, " dBFS")}
            </dd>
            <dt>Output peak / RMS</dt>
            <dd>
              {number(metrics?.outputPeakDbfs, " dBFS")} / {number(metrics?.outputRmsDbfs, " dBFS")}
            </dd>
            <dt>Clipping</dt>
            <dd>
              {metrics
                ? metrics.inputClipping || metrics.outputClipping
                  ? "Detected"
                  : "Not detected in last callback"
                : "Unavailable"}
            </dd>
            <dt>Limiter interventions</dt>
            <dd>{metrics?.protectedClipCount ?? "Unavailable"}</dd>
            <dt>Underruns / overruns</dt>
            <dd>
              {metrics ? `${metrics.underrunCount} / ${metrics.overflowCount}` : "Unavailable"}
            </dd>
            <dt>Queued / discarded frames</dt>
            <dd>
              {metrics
                ? `${metrics.ringDepthFrames} / ${metrics.discardedFrames ?? "Unavailable"}`
                : "Unavailable"}
            </dd>
            <dt>Capture / playback callback max</dt>
            <dd>
              {number(metrics?.captureCallbackMaxMicros, " µs")} /{" "}
              {number(metrics?.outputCallbackMaxMicros, " µs")}
            </dd>
            <dt>Clock correction</dt>
            <dd>{number(metrics?.driftCorrectionPpm, " ppm")}</dd>
            <dt>Pipeline latency estimate</dt>
            <dd>{number(metrics?.estimatedLatencyMs, " ms")}</dd>
            <dt>Playback FIR / limiter delay</dt>
            <dd>{metrics?.playbackAlgorithmFrames ?? "Unavailable"} frames</dd>
            <dt>Hardware loopback latency</dt>
            <dd>Not measured by this panel</dd>
            <dt>AI inference / IPC</dt>
            <dd>
              {number(ai.status?.conversionMs, " ms")} / {number(ai.status?.roundTripMs, " ms")}
            </dd>
            <dt>Configured AI budget / misses</dt>
            <dd>
              {number(ai.status?.inferenceBudgetMs, " ms")} /{" "}
              {ai.status?.missedDeadlines ?? "Unavailable"}
            </dd>
            <dt>AI missing / dropped output samples</dt>
            <dd>
              {ai.status
                ? `${ai.status.missingOutputSamples} / ${ai.status.droppedOutputSamples}`
                : "Unavailable"}
            </dd>
            <dt>Virtual routing</dt>
            <dd>
              {routing.status.state} · {number(routing.status.outputPeakDbfs, " dBFS")}
            </dd>
          </dl>
          <p className="vm-hint">
            Callback maxima are observed durations, not whole-process CPU. Latency estimate includes
            configured DSP/buffer terms and queue depth; it is not a microphone-to-Discord
            measurement.
          </p>
        </Card>
        <Card>
          <h3>Developer stage capture</h3>
          <p className="vm-hint">
            Optional local float32 WAVs. Raw input can contain private speech. AI/PCM stages require
            an active AI model; virtual output requires routing. Missing sources produce no file.
            Bypass reference is the dry input-gain reference, not a change of voice mode.
          </p>
          <div className="vm-stage-checks">
            {stages.map(([stage, label]) => (
              <label className="vm-check" key={stage}>
                <input
                  type="checkbox"
                  checked={selected.includes(stage)}
                  disabled={capture?.running || busy}
                  onChange={(e) =>
                    setSelected(
                      e.target.checked ? [...selected, stage] : selected.filter((s) => s !== stage),
                    )
                  }
                />
                {label}
              </label>
            ))}
          </div>
          <label className="vm-field">
            Capture seconds
            <input
              aria-label="Diagnostic capture seconds"
              type="number"
              min={1}
              max={30}
              value={seconds}
              disabled={capture?.running}
              onChange={(e) => setSeconds(Number(e.target.value))}
            />
          </label>
          <label className="vm-check">
            <input
              type="checkbox"
              checked={consent}
              disabled={capture?.running}
              onChange={(e) => setConsent(e.target.checked)}
            />
            I explicitly allow recording these audio stages to local WAV files.
          </label>
          <div className="vm-actions">
            <Button
              disabled={
                busy ||
                !capture ||
                capture.running ||
                !live ||
                !consent ||
                !selected.length ||
                seconds < 1 ||
                seconds > 30
              }
              onClick={() =>
                void run(async () => {
                  setCapture(await api.startStageCapture(selected, seconds, consent));
                  setConsent(false);
                })
              }
            >
              Record selected diagnostic stages
            </Button>
            <Button
              disabled={busy || !capture?.running}
              onClick={() =>
                void run(async () => {
                  setCapture(await api.stopStageCapture());
                })
              }
            >
              Stop diagnostic capture
            </Button>
          </div>
          {capture && (
            <p role="status" className="vm-hint">
              {capture.running ? "Local audio recording active. " : "Capture idle. "}
              {capture.message} Dropped diagnostic samples: {capture.droppedSamples}.
            </p>
          )}
          {capture?.folder && <p className="vm-hint">Local folder: {capture.folder}</p>}
          {capture?.files.map((file) => (
            <p className="vm-hint" key={file.stage}>
              {file.stage}: {file.sampleRate} Hz / {file.channels} ch · {file.frames} frames · peak{" "}
              {file.peak.toFixed(4)} · RMS {file.rms.toFixed(4)} · samples ≥ full scale{" "}
              {file.clippedSamples} · non-finite {file.nonFiniteSamples}
            </p>
          ))}
        </Card>
      </div>
    </section>
  );
}
