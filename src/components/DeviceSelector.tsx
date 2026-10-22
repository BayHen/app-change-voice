import { CaretDown } from "@phosphor-icons/react";
import type { DeviceOption } from "../types/audio";

interface DeviceSelectorProps {
  id: string;
  label: string;
  value: string;
  devices: DeviceOption[];
  disabled?: boolean;
  helperText?: string;
  onChange: (deviceId: string) => void;
}

export function DeviceSelector({
  id,
  label,
  value,
  devices,
  disabled = false,
  helperText,
  onChange,
}: DeviceSelectorProps) {
  return (
    <div className="space-y-2">
      <label htmlFor={id} className="block text-xs font-medium text-zinc-400">
        {label}
      </label>
      <div className="relative">
        <select
          id={id}
          value={value}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
          className="h-11 w-full appearance-none rounded-xl border border-white/[0.09] bg-[#111419] px-3 pr-10 text-sm text-zinc-200 outline-none transition-colors focus:border-cyan-400/70 disabled:cursor-not-allowed disabled:text-zinc-600"
        >
          {devices.map((device) => (
            <option key={device.id} value={device.id} disabled={!device.available}>
              {device.name}
              {device.isDefault ? " (Default)" : ""}
            </option>
          ))}
        </select>
        <CaretDown
          size={15}
          className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-zinc-600"
          aria-hidden="true"
        />
      </div>
      {helperText ? <p className="text-[11px] leading-4 text-zinc-600">{helperText}</p> : null}
    </div>
  );
}
