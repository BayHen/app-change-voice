import { Play, Square, Trash, Star } from "@phosphor-icons/react";
import type { SoundEntry, SoundRoute } from "../types/soundboard";
import { SoundVolumeSlider } from "./SoundVolumeSlider";
import { HotkeyAssignment } from "./HotkeyAssignment";
interface Props {
  sound: SoundEntry;
  busy: boolean;
  onPlay: () => void;
  onStop: () => void;
  onDelete: () => void;
  onUpdate: (sound: SoundEntry) => void;
}
export function SoundCard({ sound, busy, onPlay, onStop, onDelete, onUpdate }: Props) {
  return (
    <article className="rounded-2xl border border-white/[.08] bg-[#111419] p-4">
      <div className="flex items-start gap-3">
        <button
          aria-label={sound.favorite ? "Remove favorite" : "Add favorite"}
          onClick={() => onUpdate({ ...sound, favorite: !sound.favorite })}
        >
          <Star
            size={20}
            weight={sound.favorite ? "fill" : "regular"}
            className={sound.favorite ? "text-amber-300" : "text-zinc-600"}
          />
        </button>
        <div className="min-w-0 flex-1">
          <input
            aria-label="Sound name"
            className="w-full truncate bg-transparent font-medium text-zinc-100 outline-none focus:text-cyan-100"
            value={sound.name}
            disabled={busy}
            onChange={(event) => onUpdate({ ...sound, name: event.target.value })}
          />
          <p className="mt-1 truncate text-xs text-zinc-600">
            {sound.missing
              ? "File missing"
              : `${sound.durationSeconds.toFixed(1)}s · ${sound.category}`}
          </p>
        </div>
      </div>
      <div className="mt-4 grid grid-cols-2 gap-2">
        <button
          disabled={busy || sound.missing}
          onClick={onPlay}
          className="flex items-center justify-center gap-2 rounded-lg bg-cyan-300 px-3 py-2 text-sm font-medium text-slate-950 disabled:opacity-40"
        >
          <Play />
          Play
        </button>
        <button
          disabled={busy}
          onClick={onStop}
          className="flex items-center justify-center gap-2 rounded-lg bg-white/5 px-3 py-2 text-sm"
        >
          <Square />
          Stop
        </button>
      </div>
      <div className="mt-4 space-y-3">
        <SoundVolumeSlider
          value={sound.volumePercent}
          disabled={busy}
          onChange={(volumePercent) => onUpdate({ ...sound, volumePercent })}
        />
        <label className="block text-xs text-zinc-500">
          Category
          <input
            aria-label="Sound category"
            className="mt-2 w-full rounded-lg border border-white/10 bg-[#0c0f13] px-3 py-2 text-sm text-zinc-200"
            value={sound.category}
            disabled={busy}
            onChange={(event) => onUpdate({ ...sound, category: event.target.value })}
          />
        </label>
        <label className="block text-xs text-zinc-500">
          Route
          <select
            aria-label="Sound route"
            className="mt-2 w-full rounded-lg border border-white/10 bg-[#0c0f13] px-3 py-2 text-sm text-zinc-200"
            value={sound.route}
            onChange={(e) => onUpdate({ ...sound, route: e.target.value as SoundRoute })}
          >
            <option value="headphones">Headphones</option>
            <option value="virtualMicrophone">Virtual microphone</option>
            <option value="both">Both</option>
          </select>
        </label>
        <HotkeyAssignment
          value={sound.hotkey}
          onChange={(hotkey) => onUpdate({ ...sound, hotkey })}
        />
      </div>
      <button onClick={onDelete} className="mt-4 flex items-center gap-2 text-xs text-rose-300">
        <Trash />
        Remove reference
      </button>
    </article>
  );
}
