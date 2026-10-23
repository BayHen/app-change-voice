interface Props {
  path: string;
  name: string;
  onName: (v: string) => void;
  onBrowse: () => void;
  onAdd: () => void;
  onCancel: () => void;
  busy: boolean;
}
export function AddSoundDialog(p: Props) {
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="add-sound-title"
      className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4"
    >
      <div className="w-full max-w-lg rounded-2xl border border-white/10 bg-[#13171c] p-6">
        <h2 id="add-sound-title" className="text-lg font-semibold">
          Add WAV sound
        </h2>
        <p className="mt-2 text-sm text-zinc-500">
          Files stay local. Decoding happens before realtime playback.
        </p>
        <label className="mt-5 block text-xs text-zinc-400">
          Name
          <input
            className="mt-2 w-full rounded-lg bg-black/30 px-3 py-2"
            value={p.name}
            onChange={(e) => p.onName(e.target.value)}
          />
        </label>
        <label className="mt-4 block text-xs text-zinc-400">
          WAV file
          <div className="mt-2 flex gap-2">
            <input
              readOnly
              className="min-w-0 flex-1 rounded-lg bg-black/30 px-3 py-2 text-sm"
              value={p.path}
            />
            <button onClick={p.onBrowse} className="rounded-lg bg-white/10 px-3">
              Browse
            </button>
          </div>
        </label>
        <div className="mt-6 flex justify-end gap-2">
          <button onClick={p.onCancel} className="rounded-lg px-4 py-2 text-sm text-zinc-400">
            Cancel
          </button>
          <button
            disabled={p.busy || !p.path || !p.name.trim()}
            onClick={p.onAdd}
            className="rounded-lg bg-cyan-300 px-4 py-2 text-sm font-medium text-slate-950 disabled:opacity-40"
          >
            Add sound
          </button>
        </div>
      </div>
    </div>
  );
}
