import { Info, Warning, X } from "@phosphor-icons/react";
import { useEffect } from "react";
import type { ToastMessage } from "../types/ui";

interface NotificationToastProps {
  toast: ToastMessage | null;
  onDismiss: () => void;
}

export function NotificationToast({ toast, onDismiss }: NotificationToastProps) {
  useEffect(() => {
    if (!toast) return;
    const timeout = window.setTimeout(onDismiss, 4200);
    return () => window.clearTimeout(timeout);
  }, [onDismiss, toast]);

  if (!toast) return null;

  const ToneIcon = toast.tone === "warning" ? Warning : Info;

  return (
    <div
      role="status"
      aria-live="polite"
      className="fixed bottom-20 right-4 z-40 flex w-[min(360px,calc(100vw-2rem))] items-start gap-3 rounded-2xl border border-white/10 bg-[#15191f] p-4 shadow-[0_18px_50px_rgba(2,8,12,0.5)] md:bottom-5"
    >
      <ToneIcon size={19} className="mt-0.5 shrink-0 text-cyan-300" aria-hidden="true" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-zinc-200">{toast.title}</p>
        <p className="mt-1 text-xs leading-5 text-zinc-500">{toast.message}</p>
      </div>
      <button
        type="button"
        aria-label="Dismiss notification"
        onClick={onDismiss}
        className="grid size-7 shrink-0 place-items-center rounded-lg text-zinc-600 hover:bg-white/[0.05] hover:text-zinc-300"
      >
        <X size={14} aria-hidden="true" />
      </button>
    </div>
  );
}
