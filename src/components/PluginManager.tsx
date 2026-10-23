import type { PluginStatus } from "../types/soundboard";
export function PluginManager({
  plugins,
  onToggle,
}: {
  plugins: PluginStatus[];
  onToggle: (id: string, value: boolean) => void;
}) {
  return (
    <section>
      <h2 className="text-sm font-semibold">Trusted effect plugins</h2>
      <div className="mt-3 space-y-2">
        {plugins.map((p) => (
          <div
            key={p.manifest.id}
            className="flex items-center justify-between rounded-xl border border-white/[.07] bg-white/[.025] p-3"
          >
            <div>
              <p className="text-sm text-zinc-200">{p.manifest.name}</p>
              <p className="text-xs text-zinc-600">
                v{p.manifest.version} · {p.manifest.effects.length} effects · declarative
              </p>
            </div>
            <button
              role="switch"
              aria-checked={p.enabled}
              onClick={() => onToggle(p.manifest.id, !p.enabled)}
              className={`rounded-full px-3 py-1 text-xs ${p.enabled ? "bg-emerald-300/15 text-emerald-200" : "bg-white/5 text-zinc-500"}`}
            >
              {p.enabled ? "Enabled" : "Disabled"}
            </button>
          </div>
        ))}
      </div>
    </section>
  );
}
