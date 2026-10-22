import { VolumeSlider } from "./VolumeSlider";

export interface VoiceControlValues {
  pitch: number;
  formant: number;
  bass: number;
  treble: number;
  gain: number;
}

interface VoiceControlsProps {
  values: VoiceControlValues;
  disabled?: boolean;
  onChange: (values: VoiceControlValues) => void;
}

const controlDefinitions: Array<{
  key: keyof VoiceControlValues;
  label: string;
  min: number;
  max: number;
  unit: string;
  step?: number;
  available: boolean;
}> = [
  { key: "pitch", label: "Pitch", min: -12, max: 12, step: 0.1, unit: " st", available: true },
  { key: "formant", label: "Formant", min: -6, max: 6, step: 0.1, unit: " st", available: true },
  { key: "bass", label: "Bass — Coming Soon", min: -12, max: 12, unit: " dB", available: false },
  {
    key: "treble",
    label: "Treble — Coming Soon",
    min: -12,
    max: 12,
    unit: " dB",
    available: false,
  },
  {
    key: "gain",
    label: "Studio gain — Coming Soon",
    min: -24,
    max: 12,
    unit: " dB",
    available: false,
  },
];

export function VoiceControls({ values, disabled = false, onChange }: VoiceControlsProps) {
  return (
    <div className="grid gap-x-8 gap-y-7 sm:grid-cols-2">
      {controlDefinitions.map((control) => (
        <VolumeSlider
          key={control.key}
          id={`voice-control-${control.key}`}
          label={control.label}
          min={control.min}
          max={control.max}
          step={control.step}
          unit={control.unit}
          value={values[control.key]}
          disabled={disabled || !control.available}
          onChange={(value) => onChange({ ...values, [control.key]: value })}
        />
      ))}
    </div>
  );
}
