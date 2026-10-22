import { useCallback, useState } from "react";
import { AppSidebar } from "./components/AppSidebar";
import { NotificationToast } from "./components/NotificationToast";
import { Dashboard } from "./pages/Dashboard";
import { Settings } from "./pages/Settings";
import { VoiceEffects } from "./pages/VoiceEffects";
import { VoiceStudio } from "./pages/VoiceStudio";
import { VirtualMicrophone } from "./pages/VirtualMicrophone";
import type { NavigationItem } from "./types/navigation";
import type { VoicePresetSummary } from "./types/preset";
import type { ToastMessage } from "./types/ui";

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
