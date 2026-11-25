import * as Switch from "@radix-ui/react-switch";
export function ToggleSwitch({
  label,
  description,
  checked,
  disabled = false,
  onChange,
}: {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="vm-switch-row">
      <div>
        <p>{label}</p>
        {description && <small>{description}</small>}
      </div>
      <Switch.Root
        className="vm-switch"
        aria-label={label}
        checked={checked}
        disabled={disabled}
        onCheckedChange={onChange}
      >
        <Switch.Thumb className="vm-switch-thumb" />
      </Switch.Root>
    </div>
  );
}
