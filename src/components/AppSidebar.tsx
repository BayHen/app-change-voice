import {
  CirclesFour,
  FadersHorizontal,
  GearSix,
  MicrophoneStage,
  Waveform,
  type Icon,
} from "@phosphor-icons/react";
import type { NavigationItem } from "../types/navigation";
import { BrandMark } from "./BrandMark";

interface AppSidebarProps {
  activePage: NavigationItem;
  onNavigate: (page: NavigationItem) => void;
}

interface NavigationEntry {
  id: NavigationItem;
  label: string;
  icon: Icon;
}

const navigation: NavigationEntry[] = [
  { id: "dashboard", label: "Dashboard", icon: CirclesFour },
  { id: "effects", label: "Voice Effects", icon: Waveform },
  { id: "studio", label: "Voice Studio", icon: FadersHorizontal },
  { id: "virtual-microphone", label: "Virtual Microphone", icon: MicrophoneStage },
  { id: "settings", label: "Settings", icon: GearSix },
];

export function AppSidebar({ activePage, onNavigate }: AppSidebarProps) {
  return (
    <>
      <aside className="sticky top-0 hidden h-[100dvh] w-[232px] shrink-0 border-r border-white/[0.07] bg-[#0b0d10] px-4 py-5 md:flex md:flex-col">
        <div className="flex items-center gap-3 px-2 py-1">
          <BrandMark />
          <div>
            <p className="text-sm font-semibold tracking-[-0.02em] text-zinc-100">VoiceMorph</p>
            <p className="text-[10px] font-medium uppercase tracking-[0.18em] text-zinc-600">
              Studio
            </p>
          </div>
        </div>

        <nav className="mt-9 space-y-1" aria-label="Primary navigation">
          {navigation.map(({ id, label, icon: NavIcon }) => {
            const active = activePage === id;
            return (
              <button
                key={id}
                type="button"
                aria-current={active ? "page" : undefined}
                onClick={() => onNavigate(id)}
                className={`group flex min-h-11 w-full items-center gap-3 rounded-xl px-3 text-left text-sm transition-colors active:translate-y-px ${
                  active
                    ? "bg-cyan-300/10 font-medium text-cyan-100"
                    : "text-zinc-500 hover:bg-white/[0.04] hover:text-zinc-200"
                }`}
              >
                <NavIcon
                  size={19}
                  weight={active ? "fill" : "regular"}
                  className={active ? "text-cyan-300" : "text-zinc-600 group-hover:text-zinc-300"}
                  aria-hidden="true"
                />
                {label}
              </button>
            );
          })}
        </nav>

        <div className="mt-auto border-t border-white/[0.07] px-2 pt-4">
          <div className="flex items-center justify-between text-[11px] text-zinc-600">
            <span>Desktop build</span>
            <span className="font-mono text-zinc-500">0.1.0</span>
          </div>
          <p className="mt-1 text-[11px] text-zinc-700">Windows x64</p>
        </div>
      </aside>

      <nav
        className="fixed inset-x-3 bottom-3 z-30 grid grid-cols-5 rounded-2xl border border-white/10 bg-[#101318]/95 p-1.5 shadow-[0_14px_40px_rgba(2,8,12,0.55)] backdrop-blur md:hidden"
        aria-label="Mobile navigation"
      >
        {navigation.map(({ id, label, icon: NavIcon }) => {
          const active = activePage === id;
          return (
            <button
              key={id}
              type="button"
              aria-current={active ? "page" : undefined}
              aria-label={label}
              onClick={() => onNavigate(id)}
              className={`flex min-h-12 flex-col items-center justify-center gap-1 rounded-xl text-[10px] transition-colors ${active ? "bg-cyan-300/10 text-cyan-200" : "text-zinc-600"}`}
            >
              <NavIcon size={18} weight={active ? "fill" : "regular"} aria-hidden="true" />
              <span className="max-w-full truncate">{label}</span>
            </button>
          );
        })}
      </nav>
    </>
  );
}
