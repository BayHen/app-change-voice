import { StatusBadge } from "../components/StatusBadge";
import { useAppStatus } from "../hooks/useAppStatus";

export function Dashboard() {
  const { status, loading, error } = useAppStatus();

  return (
    <div className="mx-auto max-w-6xl">
      <header className="flex flex-col justify-between gap-5 sm:flex-row sm:items-end">
        <div>
          <p className="mb-2 text-xs font-medium uppercase tracking-[0.22em] text-violet-400">
            Audio workspace
          </p>
          <h1 className="text-3xl font-semibold tracking-[-0.04em] text-white sm:text-4xl">
            VoiceMorph Studio
          </h1>
          <p className="mt-3 max-w-xl text-sm leading-6 text-zinc-500">
            Biến đổi giọng nói trong một workspace gọn gàng, riêng tư và sẵn sàng cho real-time
            audio.
          </p>
        </div>
        <StatusBadge
          label={loading ? "Đang kết nối Rust…" : error ? "Chạy ở trình duyệt" : "Tauri đã kết nối"}
          tone={status ? "ready" : "idle"}
        />
      </header>

      <section className="mt-9 grid gap-4 lg:grid-cols-[1.5fr_1fr]">
        <div className="relative overflow-hidden rounded-3xl border border-white/8 bg-gradient-to-br from-violet-500/15 via-[#121118] to-[#0d0e12] p-7 sm:p-9">
          <div className="absolute -right-24 -top-24 size-64 rounded-full bg-violet-500/15 blur-3xl" />
          <div className="relative">
            <p className="text-xs uppercase tracking-[0.2em] text-violet-300/70">Quick start</p>
            <h2 className="mt-4 max-w-md text-2xl font-medium tracking-tight text-white">
              Bắt đầu một phiên biến đổi giọng nói mới
            </h2>
            <p className="mt-3 max-w-lg text-sm leading-6 text-zinc-400">
              Audio engine chưa được kích hoạt trong Task 01. Bạn có thể khám phá giao diện và kiểm
              tra cầu nối React ↔ Rust ngay bây giờ.
            </p>
            <button
              type="button"
              disabled
              className="mt-7 rounded-xl bg-violet-500 px-5 py-3 text-sm font-medium text-white opacity-60 shadow-[0_8px_30px_rgba(139,92,246,0.25)]"
            >
              New voice session · Soon
            </button>
          </div>
        </div>

        <div className="rounded-3xl border border-white/8 bg-[#0e0f14] p-6">
          <p className="text-xs uppercase tracking-[0.2em] text-zinc-600">System bridge</p>
          <dl className="mt-6 space-y-5">
            <div className="flex items-center justify-between gap-4">
              <dt className="text-sm text-zinc-500">Ứng dụng</dt>
              <dd className="text-sm text-zinc-200">{status?.appName ?? "VoiceMorph Studio"}</dd>
            </div>
            <div className="h-px bg-white/5" />
            <div className="flex items-center justify-between gap-4">
              <dt className="text-sm text-zinc-500">Nền tảng</dt>
              <dd className="font-mono text-xs text-zinc-300">
                {status?.platform ?? "Windows x64"}
              </dd>
            </div>
            <div className="h-px bg-white/5" />
            <div className="flex items-center justify-between gap-4">
              <dt className="text-sm text-zinc-500">Audio engine</dt>
              <dd className="text-xs text-amber-300">Chưa triển khai</dd>
            </div>
          </dl>
        </div>
      </section>

      <section className="mt-4 grid gap-4 sm:grid-cols-3">
        {[
          ["Input device", "Chưa chọn", "IN"],
          ["Active effect", "Clean voice", "FX"],
          ["Output format", "WAV · 48 kHz", "OUT"],
        ].map(([label, value, code]) => (
          <div key={label} className="rounded-2xl border border-white/8 bg-[#0e0f14] p-5">
            <div className="flex items-center justify-between">
              <p className="text-xs text-zinc-600">{label}</p>
              <span className="font-mono text-[10px] text-violet-400/70">{code}</span>
            </div>
            <p className="mt-5 text-sm font-medium text-zinc-200">{value}</p>
          </div>
        ))}
      </section>
    </div>
  );
}
