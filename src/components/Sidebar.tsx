import { BrandMark } from "./BrandMark";
import type { NavigationItem } from "../types/navigation";

interface SidebarProps {
  activePage: NavigationItem;
  onNavigate: (page: NavigationItem) => void;
}

const navigation: Array<{ id: NavigationItem; label: string; icon: string }> = [
  { id: "dashboard", label: "Dashboard", icon: "⌁" },
  { id: "effects", label: "Voice Effects", icon: "◈" },
  { id: "studio", label: "Voice Studio", icon: "◉" },
  { id: "settings", label: "Settings", icon: "⚙" },
];

export function Sidebar({ activePage, onNavigate }: SidebarProps) {
  return (
    <aside className="sticky top-0 hidden h-screen w-64 shrink-0 border-r border-white/8 bg-[#0b0c10]/90 px-5 py-7 backdrop-blur md:flex md:flex-col">
      <div className="flex items-center gap-3 px-2">
        <BrandMark />
        <div>
          <p className="text-sm font-semibold tracking-tight text-white">VoiceMorph</p>
          <p className="text-[10px] uppercase tracking-[0.24em] text-zinc-500">Studio</p>
        </div>
      </div>

      <nav className="mt-12 space-y-1" aria-label="Điều hướng chính">
        {navigation.map((item) => {
          const active = item.id === activePage;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onNavigate(item.id)}
              className={`flex w-full items-center gap-3 rounded-xl px-3 py-3 text-left text-sm transition ${
                active
                  ? "bg-white/10 font-medium text-white"
                  : "text-zinc-500 hover:bg-white/5 hover:text-zinc-200"
              }`}
            >
              <span
                className={`grid size-8 place-items-center rounded-lg text-base ${active ? "bg-violet-500/20 text-violet-300" : "bg-white/[0.03]"}`}
                aria-hidden="true"
              >
                {item.icon}
              </span>
              {item.label}
            </button>
          );
        })}
      </nav>

      <div className="mt-auto rounded-xl border border-white/8 bg-white/[0.025] p-4">
        <div className="mb-2 flex items-center justify-between text-xs">
          <span className="text-zinc-500">Phiên bản</span>
          <span className="font-mono text-zinc-300">v0.1.0</span>
        </div>
        <p className="text-[11px] leading-relaxed text-zinc-600">Windows 10/11 · x64</p>
      </div>
    </aside>
  );
}
