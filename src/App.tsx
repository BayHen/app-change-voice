import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AppSidebar } from "./components/AppSidebar";
import { NotificationToast } from "./components/NotificationToast";
import { VoiceLab } from "./pages/VoiceLab";
import { Recordings } from "./pages/Recordings";
import { AudioSessionProvider } from "./hooks/AudioSession";
import { AudioStatusBar } from "./components/AudioStatusBar";
import { AudioLines } from "lucide-react";
import { Dashboard } from "./pages/Dashboard";
import { Settings } from "./pages/Settings";
import { VoiceEffects } from "./pages/VoiceEffects";
import { VoiceStudio } from "./pages/VoiceStudio";
import { VirtualMicrophone } from "./pages/VirtualMicrophone";
import { SoundboardPage } from "./pages/SoundboardPage";
import { GameAppIntegration } from "./pages/GameAppIntegration";
import { AIVoiceStudio } from "./pages/AIVoiceStudio";
import type { NavigationItem } from "./types/navigation";
import type { VoicePresetSummary } from "./types/preset";
import type { ToastMessage } from "./types/ui";
import type { DesktopActionResult } from "./types/hotkeys";

export default function App() {
  return (
    <AudioSessionProvider>
      <StudioApp />
    </AudioSessionProvider>
  );
}

function StudioApp() {
  const [activePage, setActivePage] = useState<NavigationItem>("dashboard");
  const [toast, setToast] = useState<ToastMessage | null>(null);

  const dismissToast = useCallback(() => setToast(null), []);

  function handlePresetSelected(preset: VoicePresetSummary) {
    setToast({
      id: Date.now(),
      title: `${preset.name} selected`,
      message: "Preset validated and applied to the realtime Rust DSP chain.",
      tone: "info",
    });
  }

  const notify = useCallback((title: string, message: string) => {
    setToast({ id: Date.now(), title, message, tone: "warning" });
  }, []);

  useEffect(() => {
    const disposers: Array<() => void> = [];
    let active = true;
    void listen<NavigationItem>("voicemorph://navigate", (event) => setActivePage(event.payload))
      .then((dispose) => (active ? disposers.push(dispose) : dispose()))
      .catch(() => undefined);
    void listen<DesktopActionResult>("voicemorph://action", (event) => {
      setToast({
        id: Date.now(),
        title: "Desktop action",
        message: event.payload.message,
        tone: "info",
      });
    })
      .then((dispose) => (active ? disposers.push(dispose) : dispose()))
      .catch(() => undefined);
    void listen<DesktopActionResult>("voicemorph://action-error", (event) => {
      setToast({
        id: Date.now(),
        title: "Desktop action failed",
        message: event.payload.message,
        tone: "warning",
      });
    })
      .then((dispose) => (active ? disposers.push(dispose) : dispose()))
      .catch(() => undefined);
    return () => {
      active = false;
      disposers.forEach((dispose) => dispose());
    };
  }, []);

  function renderPage() {
    switch (activePage) {
      case "recordings":
        return <Recordings />;
      case "effects":
        return <VoiceEffects onPresetSelected={handlePresetSelected} />;
      case "studio":
        return <VoiceStudio />;
      case "settings":
        return (
          <Settings
            advanced={<Dashboard onNotify={notify} />}
            effects={<VoiceEffects onPresetSelected={handlePresetSelected} />}
            studio={<VoiceStudio />}
            routing={<VirtualMicrophone />}
            integrations={<GameAppIntegration />}
          />
        );
      case "virtual-microphone":
        return <VirtualMicrophone />;
      case "soundboard":
        return <SoundboardPage />;
      case "integrations":
        return <GameAppIntegration />;
      case "ai-studio":
        return <AIVoiceStudio />;
      default:
        return <VoiceLab />;
    }
  }

  return (
    <div className="vm-app">
      <div className="vm-titlebar">
        <span>
          <AudioLines size={14} /> VoiceMorph Studio
        </span>
        <span>Realtime voice workspace</span>
      </div>
      <div className="vm-shell">
        <AppSidebar activePage={activePage} onNavigate={setActivePage} />
        <main className="vm-workspace" id="main-content">
          {renderPage()}
        </main>
      </div>
      <AudioStatusBar onSettings={() => setActivePage("settings")} />
      <NotificationToast toast={toast} onDismiss={dismissToast} />
    </div>
  );
}
