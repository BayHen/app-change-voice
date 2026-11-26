import { useEffect, useMemo, useState } from "react";
import { useAudioSession } from "../hooks/AudioSession";
import { errorMessage } from "../services/errors";
import { AppHeader } from "../components/AppHeader";
import { Button, Card, EmptyState, SearchInput, Badge } from "../components/ui";
import { open } from "@tauri-apps/plugin-dialog";
import { Plus, Warning } from "../components/icons";
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
  const { runtime, devices } = useAudioSession();
  const live = runtime.status.state === "running";
  const input = devices.snapshot?.inputSelection.selectedId,
    output = devices.snapshot?.outputSelection.selectedId;
  const [sounds, setSounds] = useState<SoundEntry[]>([]),
    [plugins, setPlugins] = useState<PluginStatus[]>([]),
    [effects, setEffects] = useState<EffectDescriptor[]>([]),
    [query, setQuery] = useState(""),
    [dialog, setDialog] = useState(false),
    [path, setPath] = useState(""),
    [name, setName] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [category, setCategory] = useState("All sounds");
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
      .catch((e) => setError(errorMessage(e)));
  }, []);
  const filtered = useMemo(
    () =>
      sounds.filter(
        (s) =>
          `${s.name} ${s.category}`.toLowerCase().includes(query.toLowerCase()) &&
          (category === "All sounds" ||
            (category === "Favorites" ? s.favorite : s.category === category)),
      ),
    [sounds, query, category],
  );
  async function mutate(task: () => Promise<unknown>) {
    setBusy(true);
    setError("");
    try {
      await task();
      await refresh();
      return true;
    } catch (e) {
      setError(errorMessage(e));
      return false;
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="vm-page vm-soundboard">
      <AppHeader
        title="Soundboard"
        description="The perfect sound. Right on cue."
        action={
          <Button variant="primary" onClick={() => setDialog(true)}>
            <Plus size={16} />
            Import audio
          </Button>
        }
      />
      <div className="vm-toolbar">
        <SearchInput label="Search sounds" value={query} onChange={setQuery} />
        <Badge>{sounds.length} clips · WAV library</Badge>
      </div>
      <div className="vm-settings-tabs" role="group" aria-label="Sound categories">
        {[
          "All sounds",
          "Favorites",
          ...new Set(
            sounds.map((s) => s.category).filter((c) => c !== "All sounds" && c !== "Favorites"),
          ),
        ].map((c) => (
          <button
            type="button"
            key={c}
            aria-pressed={category === c}
            onClick={() => setCategory(c)}
          >
            {c}
          </button>
        ))}
      </div>
      {!live && (
        <Card>
          <h2>Start audio processing to play sounds</h2>
          <p className="vm-hint">
            Soundboard uses the audio engine. Starting it opens your selected microphone and output
            device. Virtual-microphone clips also need Start routing in Settings.
          </p>
          <Button
            disabled={busy || runtime.busy || !!devices.error || !input || !output}
            onClick={() => {
              if (input && output) void runtime.start(input, output);
            }}
          >
            Start audio engine
          </Button>
        </Card>
      )}
      {(error || runtime.error) && (
        <p
          role="alert"
          className="mt-4 flex gap-2 rounded-xl bg-rose-400/10 p-3 text-sm text-rose-200"
        >
          <Warning /> {error || runtime.error}
        </p>
      )}
      <div className="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {filtered.map((sound) => (
          <SoundCard
            key={sound.id}
            sound={sound}
            busy={busy}
            playbackDisabled={!live || runtime.busy}
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
        <Card>
          <EmptyState
            title={
              query || category !== "All sounds"
                ? "No sounds in this view"
                : "Give your stream a soundtrack"
            }
            description="Import a WAV clip, assign a shortcut and send it to your headphones, virtual microphone, or both."
            action={
              <Button onClick={() => setDialog(true)}>
                <Plus size={15} />
                Import your first sound
              </Button>
            }
          />
        </Card>
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
            })
              .then((v) => {
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
              .catch((e) => setError(errorMessage(e)))
          }
          onCancel={() => setDialog(false)}
          onAdd={() =>
            void mutate(() => addSound(path, name)).then((success) => {
              if (!success) return;
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
