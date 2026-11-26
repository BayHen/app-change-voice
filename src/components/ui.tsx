import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from "react";
import { useEffect, useRef } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X, Search, AudioLines } from "lucide-react";

export function Button({
  className = "",
  variant = "secondary",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
}) {
  return (
    <button type="button" className={`vm-button vm-button-${variant} ${className}`} {...props} />
  );
}
export function Card({ className = "", ...props }: HTMLAttributes<HTMLElement>) {
  return <section className={`vm-card ${className}`} {...props} />;
}
export function Badge({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "success" | "warning";
}) {
  return <span className={`vm-badge vm-badge-${tone}`}>{children}</span>;
}
export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="vm-empty">
      <span className="vm-empty-icon">
        <AudioLines size={30} />
      </span>
      <h3>{title}</h3>
      <p>{description}</p>
      {action}
    </div>
  );
}
export function SearchInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="vm-search">
      <Search size={17} aria-hidden />
      <input
        aria-label={label}
        placeholder={label}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
}
export function StudioDialog({
  open,
  onOpenChange,
  title,
  children,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  children: ReactNode;
}) {
  const returnFocus = useRef<HTMLElement | null>(
    typeof document === "undefined" ? null : (document.activeElement as HTMLElement),
  );
  useEffect(() => {
    const remember = (event: FocusEvent) => {
      const target = event.target;
      if (target instanceof HTMLElement && !target.closest('[role="dialog"]'))
        returnFocus.current = target;
    };
    document.addEventListener("focusin", remember);
    return () => document.removeEventListener("focusin", remember);
  }, []);
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="vm-overlay" />
        <Dialog.Content
          className="vm-dialog"
          aria-describedby={undefined}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            returnFocus.current?.focus();
          }}
        >
          <div className="vm-section-heading">
            <Dialog.Title>{title}</Dialog.Title>
            <Dialog.Close asChild>
              <Button variant="ghost" aria-label="Close dialog">
                <X size={18} />
              </Button>
            </Dialog.Close>
          </div>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  unit = "",
  disabled,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  unit?: string;
  disabled?: boolean;
  onChange: (value: number) => void;
}) {
  return (
    <label className="vm-slider">
      <span>
        {label}
        <span className="vm-slider-value">
          {value.toFixed(step < 1 ? 1 : 0)}
          {unit}
        </span>
      </span>
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        style={{
          background: `linear-gradient(to right, var(--primary) ${(100 * (value - min)) / (max - min)}%, var(--control) 0)`,
        }}
      />
    </label>
  );
}
