import { useState } from "react";
import {
  AudioLines,
  ArrowDownToLine,
  Check,
  CircleUserRound,
  Headphones,
  Mic,
  Plus,
  Radio,
  Save,
  ShieldCheck,
  Sparkles,
  Bot,
  Skull,
  SlidersHorizontal,
} from "lucide-react";
import { AppHeader } from "../components/AppHeader";
import { Button, Card, Badge, Slider, StudioDialog } from "../components/ui";
import { InputLevelMeter } from "../components/InputLevelMeter";
import { Waveform } from "../components/Waveform";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { useAudioSession } from "../hooks/AudioSession";
import { useEffectParameters } from "../hooks/useEffectParameters";
import { useNoiseSettings } from "../hooks/useNoiseSettings";
import { usePresets } from "../hooks/usePresets";
import { useVolumeSettings } from "../hooks/useVolumeSettings";
import { getEffectParameters, previewPreset } from "../services/tauri";
import type { VoicePresetSummary } from "../types/preset";

function studioPreset(name: string, pitch = 0, formant = 0): VoicePresetSummary {
  return {
    schemaVersion: 2,
    id: "custom",
    name,
    description: "Local DSP voice preset",
    category: "Character",
    pitchSemitones: pitch,
    formantSemitones: formant,
    gain: { makeupDb: 0 },
    effects: [],
    builtIn: false,
    createdAt: "",
    updatedAt: "",
    favorite: false,
    parameters: {
      inputGainPercent: 100,
      outputGainPercent: 100,
      bassDb: 0,
      trebleDb: 0,
      noiseSuppression: false,
      suppressionStrength: 0.8,
      noiseGate: true,
      gateThresholdDbfs: -52,
      limiterEnabled: true,
      limiterCeilingDbfs: -1,
      effectBypassed: false,
    },
    pluginRequirements: [],
  };
}
const templates = [
  {
    name: "Deep Voice",
    detail: "Low & resonant",
    icon: AudioLines,
    pitch: -4,
    formant: -2,
    color: "violet",
  },
  { name: "Robot", detail: "Synthetic & metallic", icon: Bot, pitch: 0, formant: 0, color: "cyan" },
  {
    name: "Female Lift",
    detail: "Bright & expressive",
    icon: CircleUserRound,
    pitch: 3,
    formant: 2,
    color: "pink",
  },
  {
    name: "Stream Clean",
    detail: "Clear & focused",
    icon: Radio,
    pitch: 0,
    formant: 0,
    color: "cyan",
  },
  {
    name: "Monster",
    detail: "Dark & powerful",
    icon: Skull,
    pitch: -8,
    formant: -4,
    color: "orange",
  },
  {
    name: "Natural Boost",
    detail: "You, with more presence",
    icon: Sparkles,
    pitch: 0,
    formant: 0,
    color: "green",
  },
];

export function VoiceLab() {
  const { runtime, devices, routing } = useAudioSession();
  const effects = useEffectParameters(),
    noise = useNoiseSettings(),
    library = usePresets(),
    volume = useVolumeSettings();
  const [draftName, setDraftName] = useState("My studio voice"),
    [dialog, setDialog] = useState(false),
    [busy, setBusy] = useState(false),
    [message, setMessage] = useState("");
  const connected = !devices.loading && !devices.error && !!devices.snapshot;
  const live = runtime.status.state === "running";
  const parameters = effects.parameters;
  // Derive selection from the actual DSP state so navigation never loses the active voice.
  const selected =
    effects.loading || effects.error
      ? null
      : templates.find(
          (template) =>
            template.pitch === parameters.pitchSemitones &&
            template.formant === parameters.formantSemitones &&
            (template.name === "Robot"
              ? Math.abs(parameters.ringModMix - 0.6) < 0.01
              : parameters.ringModMix === 0) &&
            (template.name === "Stream Clean") === noise.settings.enabled &&
            parameters.makeupGainDb === (template.name === "Natural Boost" ? 3 : 0),
        )?.name;
  async function applyTemplate(template: (typeof templates)[number]) {
    setBusy(true);
    setMessage("");
    try {
      const preset = studioPreset(template.name, template.pitch, template.formant);
      if (template.name === "Robot")
        preset.effects = [{ type: "ringModulator", frequencyHz: 72, mix: 0.6 }];
      if (template.name === "Stream Clean") preset.parameters.noiseSuppression = true;
      if (template.name === "Natural Boost") preset.gain.makeupDb = 3;
      await previewPreset(preset);
      await effects.update(await getEffectParameters());
      await noise.commit({ ...noise.settings, enabled: preset.parameters.noiseSuppression });
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }
  async function savePreset() {
    setBusy(true);
    const preset = studioPreset(draftName, parameters.pitchSemitones, parameters.formantSemitones);
    preset.gain.makeupDb = parameters.makeupGainDb;
    preset.parameters = {
      ...preset.parameters,
      inputGainPercent: volume.settings.inputPercent,
      outputGainPercent: volume.settings.outputPercent,
      bassDb: parameters.bassDb,
      trebleDb: parameters.trebleDb,
      noiseSuppression: noise.settings.enabled,
      suppressionStrength: noise.settings.suppressionStrength,
      noiseGate: noise.settings.gateEnabled,
      gateThresholdDbfs: noise.settings.gateThresholdDbfs,
      limiterEnabled: noise.settings.limiterEnabled,
      limiterCeilingDbfs: noise.settings.limiterCeilingDbfs,
      effectBypassed: parameters.bypassed,
    };
    preset.effects = [
      {
        type: "ringModulator",
        frequencyHz: parameters.ringModFrequencyHz,
        mix: parameters.ringModMix,
      },
      {
        type: "radioFilter",
        highPassHz: parameters.radioHighPassHz,
        lowPassHz: parameters.radioLowPassHz,
        drive: parameters.radioDrive,
        mix: parameters.radioMix,
      },
      { type: "tremolo", rateHz: parameters.tremoloRateHz, depth: parameters.tremoloDepth },
    ];
    if (await library.create(preset)) {
      setDialog(false);
      setMessage("Preset saved to your local library.");
    }
    setBusy(false);
  }
  return (
    <div className="vm-page">
      <AppHeader
        title="Voice Lab"
        description="Your voice. A whole new possibility."
        action={
          <div className="vm-actions">
            <Button disabled={!connected || busy} onClick={() => setDialog(true)}>
              <Save size={15} />
              Save preset
            </Button>
            <Button
              title="Export the active saved library preset. Save a custom voice first to retain edits."
              disabled={!(library.lastCreatedId ?? library.activeId) || busy}
              onClick={() => {
                const id = library.lastCreatedId ?? library.activeId;
                if (id) void library.exportJson(id);
              }}
            >
              <ArrowDownToLine size={15} />
              Export
            </Button>
            <Button
              variant="primary"
              disabled={!connected}
              onClick={() => {
                setDraftName("My studio voice");
                setDialog(true);
              }}
            >
              <Plus size={16} />
              New preset
            </Button>
          </div>
        }
      />
      <div className="vm-lab-grid">
        <div className="vm-lab-main">
          <Card>
            <div className="vm-section-heading">
              <div>
                <h2>Voice presets</h2>
                <p>Find your sound. Make it yours.</p>
              </div>
              <Badge>Realtime DSP</Badge>
            </div>
            <div className="vm-preset-grid">
              {templates.map((template) => {
                const Icon = template.icon;
                return (
                  <button
                    key={template.name}
                    className={`vm-preset ${selected === template.name ? "is-selected" : ""}`}
                    aria-pressed={selected === template.name}
                    disabled={!connected || busy}
                    onClick={() => void applyTemplate(template)}
                  >
                    <span className={`vm-preset-art art-${template.color}`}>
                      <Icon size={35} strokeWidth={1.4} />
                      <span className="vm-mini-bars" aria-hidden>
                        <i />
                        <i />
                        <i />
                        <i />
                        <i />
                        <i />
                        <i />
                      </span>
                    </span>
                    <strong>{template.name}</strong>
                    <span>{template.detail}</span>
                    {selected === template.name && <Check className="vm-preset-check" size={15} />}
                  </button>
                );
              })}
            </div>
          </Card>
          <Card className="vm-signal">
            <div className="vm-section-heading">
              <div>
                <h2>
                  <Mic size={16} />
                  Microphone signal
                </h2>
                <p>Live input envelope</p>
              </div>
              <Badge tone={live ? "success" : "neutral"}>
                {live ? "Live input" : "Awaiting signal"}
              </Badge>
            </div>
            <Waveform level={(runtime.status.metrics?.inputLevel ?? 0) * 100} active={live} />
            <div className="vm-wave-axis">
              <span>−∞ dB</span>
              <span>Peak / RMS history</span>
              <span>
                {runtime.status.metrics
                  ? `${runtime.status.metrics.inputPeakDbfs.toFixed(1)} dBFS`
                  : "No input"}
              </span>
            </div>
          </Card>
          <div className="vm-dsp-grid">
            <Card>
              <div className="vm-section-heading">
                <h2>
                  <SlidersHorizontal size={16} />
                  Voice shaping
                </h2>
                <Button
                  variant="ghost"
                  disabled={!connected || busy}
                  onClick={() => {
                    void effects.reset();
                  }}
                >
                  Reset
                </Button>
              </div>
              <Slider
                label="Pitch"
                min={-12}
                max={12}
                step={0.1}
                unit=" st"
                value={parameters.pitchSemitones}
                disabled={!connected || effects.loading}
                onChange={(v) => void effects.update({ ...parameters, pitchSemitones: v })}
              />
              <Slider
                label="Formant"
                min={-6}
                max={6}
                step={0.1}
                unit=" st"
                value={parameters.formantSemitones}
                disabled={!connected || effects.loading}
                onChange={(v) => void effects.update({ ...parameters, formantSemitones: v })}
              />
              <ToggleSwitch
                label="Apply voice effects"
                checked={parameters.enabled && !parameters.bypassed}
                disabled={!connected}
                onChange={(enabled) =>
                  void effects.update({ ...parameters, enabled, bypassed: false })
                }
              />
            </Card>
            <Card>
              <div className="vm-section-heading">
                <h2>
                  <AudioLines size={16} />
                  Equalizer
                </h2>
                <Badge>2-band shelf</Badge>
              </div>
              <Slider
                label="Bass"
                min={-12}
                max={12}
                unit=" dB"
                value={parameters.bassDb}
                disabled={!connected}
                onChange={(v) => void effects.update({ ...parameters, bassDb: v })}
              />
              <Slider
                label="Treble"
                min={-12}
                max={12}
                unit=" dB"
                value={parameters.trebleDb}
                disabled={!connected}
                onChange={(v) => void effects.update({ ...parameters, trebleDb: v })}
              />
              <Slider
                label="Makeup gain"
                min={-12}
                max={6}
                unit=" dB"
                value={parameters.makeupGainDb}
                disabled={!connected}
                onChange={(v) => void effects.update({ ...parameters, makeupGainDb: v })}
              />
            </Card>
          </div>
        </div>
        <aside className="vm-lab-aside">
          <Card>
            <div className="vm-section-heading">
              <h2>
                <Headphones size={17} />
                Audio devices
              </h2>
              <Button
                variant="ghost"
                aria-label="Refresh audio devices"
                onClick={() => void devices.refresh()}
              >
                <Radio size={15} />
              </Button>
            </div>
            {(["input", "output"] as const).map((kind) => (
              <label className="vm-field" key={kind}>
                {kind === "input" ? "Microphone input" : "Audio output"}
                <select
                  aria-label={kind === "input" ? "Microphone input" : "Audio output"}
                  disabled={!connected || devices.refreshing || live}
                  value={
                    kind === "input"
                      ? (devices.snapshot?.inputSelection.selectedId ?? "")
                      : (devices.snapshot?.outputSelection.selectedId ?? "")
                  }
                  onChange={(e) => void devices.select(kind, e.target.value)}
                >
                  <option value="">
                    {connected ? "Select device" : "Desktop bridge unavailable"}
                  </option>
                  {(kind === "input"
                    ? devices.snapshot?.inputDevices
                    : devices.snapshot?.outputDevices
                  )?.map((d) => (
                    <option key={d.id} value={d.id}>
                      {d.name}
                    </option>
                  ))}
                </select>
              </label>
            ))}
            <div className="vm-input-meter">
              <InputLevelMeter
                disabled={!live}
                peakDbfs={runtime.status.metrics?.inputPeakDbfs ?? -60}
                rmsDbfs={runtime.status.metrics?.inputRmsDbfs ?? -60}
              />
            </div>
            <p className="vm-hint">Use headphones while monitoring.</p>
            <div className="vm-device-status">
              <Mic size={16} />
              <span>Virtual microphone</span>
              <Badge tone={routing.status.state === "active" ? "success" : "neutral"}>
                {routing.status.state === "active" ? "Live" : "Not active"}
              </Badge>
            </div>
          </Card>
          <Card>
            <div className="vm-section-heading">
              <h2>
                <ShieldCheck size={17} />
                Noise cancellation
              </h2>
            </div>
            <ToggleSwitch
              label="Noise suppression"
              checked={noise.settings.enabled}
              disabled={!connected || noise.loading}
              onChange={(enabled) => void noise.commit({ ...noise.settings, enabled })}
            />
            <p className="vm-hint">Keep the conversation. Leave the noise.</p>
            <Slider
              label="Suppression"
              min={0}
              max={1}
              step={0.05}
              value={noise.settings.suppressionStrength}
              disabled={!connected || !noise.settings.enabled}
              onChange={(v) => void noise.commit({ ...noise.settings, suppressionStrength: v })}
            />
            <ToggleSwitch
              label="Noise gate"
              checked={noise.settings.gateEnabled}
              disabled={!connected}
              onChange={(gateEnabled) => void noise.commit({ ...noise.settings, gateEnabled })}
            />
            <ToggleSwitch
              label="Safety limiter"
              checked={noise.settings.limiterEnabled}
              disabled={!connected}
              onChange={(limiterEnabled) =>
                void noise.commit({ ...noise.settings, limiterEnabled })
              }
            />
          </Card>
          <div className="vm-local-note">
            <ShieldCheck size={20} />
            <div>
              <strong>Made for your microphone.</strong>
              <p>DSP processing stays on this device.</p>
            </div>
          </div>
          {!connected && (
            <p className="vm-offline-note" role="status">
              Open the desktop app to connect your microphone and activate audio controls.
            </p>
          )}
        </aside>
      </div>
      {connected && (message || effects.error || library.error || noise.error) ? (
        <p role="status" className="vm-notice">
          {message || effects.error || library.error || noise.error}
        </p>
      ) : null}
      <StudioDialog open={dialog} onOpenChange={setDialog} title="Save your voice">
        <p className="vm-hint">
          Save the current DSP, equalizer and noise settings to a local preset.
        </p>
        <label className="vm-field">
          Preset name
          <input
            value={draftName}
            onChange={(e) => setDraftName(e.target.value)}
            autoFocus
            maxLength={80}
          />
        </label>
        <Button
          variant="primary"
          disabled={busy || !draftName.trim()}
          onClick={() => void savePreset()}
        >
          <Save size={16} />
          {busy ? "Saving…" : "Save preset"}
        </Button>
      </StudioDialog>
    </div>
  );
}
