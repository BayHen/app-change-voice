import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Plus, MagnifyingGlass, Warning } from "@phosphor-icons/react";
import { AddSoundDialog } from "../components/AddSoundDialog";
import { SoundCard } from "../components/SoundCard";
import { PluginManager } from "../components/PluginManager";
import { EffectBrowser } from "../components/EffectBrowser";
import type { EffectDescriptor, PluginStatus, SoundEntry } from "../types/soundboard";
import {
  addSound,
  getPlugins,
  getRegisteredEffects,
  getSoundboardSounds,
  playSound,
  removeSound,
  setPluginEnabled,
  stopSound,
  updateSound,
} from "../services/tauri";
export function SoundboardPage() {
  const [sounds, setSounds] = useState<SoundEntry[]>([]),
    [plugins, setPlugins] = useState<PluginStatus[]>([]),
    [effects, setEffects] = useState<EffectDescriptor[]>([]),
    [query, setQuery] = useState(""),
    [dialog, setDialog] = useState(false),
    [path, setPath] = useState(""),
    [name, setName] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const refresh = async () => {
    const [s, p, e] = await Promise.all([
      getSoundboardSounds(),
      getPlugins(),
      getRegisteredEffects(),
    ]);
    setSounds(s);
    setPlugins(p);
    setEffects(e);
  };
  useEffect(() => {
    void Promise.all([getSoundboardSounds(), getPlugins(), getRegisteredEffects()])
      .then(([s, p, e]) => {
        setSounds(s);
        setPlugins(p);
        setEffects(e);
      })
      .catch((e) => setError(String(e)));
  }, []);
  const filtered = useMemo(
    () =>
      sounds.filter((s) => `${s.name} ${s.category}`.toLowerCase().includes(query.toLowerCase())),
    [sounds, query],
  );
  async function mutate(task: () => Promise<unknown>) {
    setBusy(true);
    setError("");
    try {
      await task();
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="mx-auto max-w-6xl">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-[.2em] text-cyan-300">Local audio launcher</p>
          <h1 className="mt-2 text-3xl font-semibold">Soundboard</h1>
          <p className="mt-2 text-sm text-zinc-500">
            Mix WAV clips into headphones, the virtual microphone, or both.
          </p>
        </div>
        <button
          onClick={() => setDialog(true)}
          className="flex items-center gap-2 rounded-xl bg-cyan-300 px-4 py-2.5 text-sm font-semibold text-slate-950"
        >
          <Plus />
          Add sound
        </button>
      </header>
      <div className="mt-7 flex items-center gap-2 rounded-xl border border-white/[.08] bg-white/[.025] px-3">
        <MagnifyingGlass className="text-zinc-600" />
        <input
          aria-label="Search sounds"
          className="w-full bg-transparent py-3 text-sm outline-none"
          placeholder="Search sounds or categories"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      {error && (
        <p
          role="alert"
          className="mt-4 flex gap-2 rounded-xl bg-rose-400/10 p-3 text-sm text-rose-200"
        >
          <Warning /> {error}
        </p>
      )}
      <div className="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {filtered.map((sound) => (
          <SoundCard
            key={sound.id}
            sound={sound}
            busy={busy}
            onPlay={() => void mutate(() => playSound(sound.id))}
            onStop={() => void mutate(() => stopSound(sound.id))}
            onDelete={() => {
              if (confirm(`Remove ${sound.name}? The WAV file will not be deleted.`))
                void mutate(() => removeSound(sound.id));
            }}
            onUpdate={(entry) => void mutate(() => updateSound(entry))}
          />
        ))}
      </div>
      {!filtered.length && (
        <p className="mt-10 text-center text-sm text-zinc-600">No sounds in this view.</p>
      )}
      <div className="mt-10 grid gap-6 border-t border-white/[.07] pt-7 lg:grid-cols-2">
        <PluginManager
          plugins={plugins}
          onToggle={(id, value) => void mutate(() => setPluginEnabled(id, value))}
        />
        <EffectBrowser effects={effects} />
      </div>
      {dialog && (
        <AddSoundDialog
          path={path}
          name={name}
          busy={busy}
          onName={setName}
          onBrowse={() =>
            void open({
              multiple: false,
              filters: [{ name: "WAV audio", extensions: ["wav"] }],
            }).then((v) => {
              if (typeof v === "string") {
                setPath(v);
                setName(
                  v
                    .split(/[\\/]/)
                    .pop()
                    ?.replace(/\.wav$/i, "") ?? "",
                );
              }
            })
          }
          onCancel={() => setDialog(false)}
          onAdd={() =>
            void mutate(() => addSound(path, name)).then(() => {
              setDialog(false);
              setPath("");
              setName("");
            })
          }
        />
      )}
    </div>
  );
}
