import type { EffectDescriptor } from "../types/soundboard";
export function EffectBrowser({ effects }: { effects: EffectDescriptor[] }) {
  return (
    <section>
      <h2 className="text-sm font-semibold">Registered effects</h2>
      <div className="mt-3 flex flex-wrap gap-2">
        {effects.map((e) => (
          <span
            key={e.id}
            title={e.id}
            className="rounded-lg border border-cyan-300/15 bg-cyan-300/[.06] px-3 py-2 text-xs text-cyan-100"
          >
            {e.name} · realtime safe
          </span>
        ))}
      </div>
    </section>
  );
}
