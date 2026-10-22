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
}> = [
  { key: "pitch", label: "Pitch", min: -12, max: 12, unit: " st" },
  { key: "formant", label: "Formant", min: -20, max: 20, unit: "%" },
  { key: "bass", label: "Bass", min: -12, max: 12, unit: " dB" },
  { key: "treble", label: "Treble", min: -12, max: 12, unit: " dB" },
  { key: "gain", label: "Output gain", min: -24, max: 12, unit: " dB" },
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
          unit={control.unit}
          value={values[control.key]}
          disabled={disabled}
          onChange={(value) => onChange({ ...values, [control.key]: value })}
        />
      ))}
    </div>
  );
}
