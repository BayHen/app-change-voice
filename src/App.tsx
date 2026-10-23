import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AppSidebar } from "./components/AppSidebar";
import { NotificationToast } from "./components/NotificationToast";
import { Dashboard } from "./pages/Dashboard";
import { Settings } from "./pages/Settings";
import { VoiceEffects } from "./pages/VoiceEffects";
import { VoiceStudio } from "./pages/VoiceStudio";
import { VirtualMicrophone } from "./pages/VirtualMicrophone";
import { SoundboardPage } from "./pages/SoundboardPage";
import { GameAppIntegration } from "./pages/GameAppIntegration";
import type { NavigationItem } from "./types/navigation";
import type { VoicePresetSummary } from "./types/preset";
import type { ToastMessage } from "./types/ui";
import type { DesktopActionResult } from "./types/hotkeys";

export default function App() {
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
      case "effects":
        return <VoiceEffects onPresetSelected={handlePresetSelected} />;
      case "studio":
        return <VoiceStudio />;
      case "settings":
        return <Settings />;
      case "virtual-microphone":
        return <VirtualMicrophone />;
      case "soundboard":
        return <SoundboardPage />;
      case "integrations":
        return <GameAppIntegration />;
      default:
        return <Dashboard onNotify={notify} />;
    }
  }

  return (
    <div className="min-h-[100dvh] bg-[#080a0d] text-zinc-100">
      <div className="mx-auto flex min-h-[100dvh] max-w-[1600px]">
        <AppSidebar activePage={activePage} onNavigate={setActivePage} />
        <main className="min-w-0 flex-1 px-4 pb-24 pt-6 sm:px-6 md:pb-8 lg:px-9 lg:pt-8">
          {renderPage()}
        </main>
      </div>
      <NotificationToast toast={toast} onDismiss={dismissToast} />
    </div>
  );
}
