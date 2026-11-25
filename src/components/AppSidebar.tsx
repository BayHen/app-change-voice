import {
  AudioLines,
  BrainCircuit,
  Disc3,
  FileAudio as FolderAudio,
  Settings2,
  ShieldCheck,
} from "lucide-react";
import type { NavigationItem } from "../types/navigation";
const navigation = [
  { id: "dashboard", label: "Voice Lab", icon: AudioLines },
  { id: "soundboard", label: "Soundboard", icon: Disc3 },
  { id: "ai-studio", label: "AI Voice Studio", icon: BrainCircuit },
  { id: "recordings", label: "Recordings", icon: FolderAudio },
  { id: "settings", label: "Settings", icon: Settings2 },
] as const;
export function AppSidebar({
  activePage,
  onNavigate,
}: {
  activePage: NavigationItem;
  onNavigate: (page: NavigationItem) => void;
}) {
  return (
    <aside className="vm-sidebar">
      <a
        className="vm-brand"
        href="#"
        onClick={(e) => {
          e.preventDefault();
          onNavigate("dashboard");
        }}
      >
        <span className="vm-brand-mark">
          <AudioLines size={28} />
        </span>
        <span>
          <strong>VoiceMorph</strong>
          <small>STUDIO</small>
        </span>
      </a>
      <div className="vm-sidebar-label">Workspace</div>
      <nav aria-label="Primary navigation">
        {navigation.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            aria-label={label}
            aria-current={activePage === id ? "page" : undefined}
            onClick={() => onNavigate(id)}
            className={activePage === id ? "is-active" : ""}
          >
            <Icon size={20} strokeWidth={1.7} />
            <span>{label}</span>
            {id === "ai-studio" && <small aria-hidden>AI</small>}
          </button>
        ))}
      </nav>
      <div className="vm-sidebar-bottom">
        <div className="vm-privacy">
          <ShieldCheck size={19} />
          <div>
            <strong>Your voice stays yours.</strong>
            <p>Local audio processing</p>
          </div>
        </div>
        <span>
          VoiceMorph Studio <span>0.1.0</span>
        </span>
      </div>
    </aside>
  );
}
