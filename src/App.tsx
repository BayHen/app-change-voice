import { useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./pages/Dashboard";
import { PlaceholderPage } from "./pages/PlaceholderPage";
import type { NavigationItem } from "./types/navigation";

const pages: Record<NavigationItem, { eyebrow: string; title: string; description: string }> = {
  dashboard: {
    eyebrow: "Tổng quan",
    title: "Dashboard",
    description: "Theo dõi thiết bị, phiên thu và trạng thái xử lý giọng nói.",
  },
  effects: {
    eyebrow: "Thư viện",
    title: "Voice Effects",
    description: "Hiệu ứng giọng nói sẽ được xây dựng trong các task tiếp theo.",
  },
  studio: {
    eyebrow: "Workspace",
    title: "Voice Studio",
    description: "Không gian thu, nghe thử và xuất âm thanh đang được chuẩn bị.",
  },
  settings: {
    eyebrow: "Hệ thống",
    title: "Settings",
    description: "Cấu hình thiết bị, hiệu năng và giao diện ứng dụng.",
  },
};

export default function App() {
  const [activePage, setActivePage] = useState<NavigationItem>("dashboard");

  return (
    <div className="min-h-screen bg-[#07080b] text-zinc-100">
      <div className="mx-auto flex min-h-screen max-w-[1600px]">
        <Sidebar activePage={activePage} onNavigate={setActivePage} />
        <main className="min-w-0 flex-1 px-6 py-7 sm:px-8 lg:px-12 lg:py-10">
          {activePage === "dashboard" ? <Dashboard /> : <PlaceholderPage {...pages[activePage]} />}
        </main>
      </div>
    </div>
  );
}
