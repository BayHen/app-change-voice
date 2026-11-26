import { useState } from "react";
import { Circle, Download, FileAudio, Play, Square, Trash2 } from "lucide-react";
import { AppHeader } from "../components/AppHeader";
import { Badge, Button, Card, EmptyState, SearchInput, StudioDialog } from "../components/ui";
import { useAudioSession } from "../hooks/AudioSession";
import { RecordingPanel } from "../components/RecordingPanel";

export function Recordings() {
  const { runtime, recording } = useAudioSession();
  const [query, setQuery] = useState(""),
    [confirmDelete, setConfirmDelete] = useState(false);
  const status = recording.status;
  return (
    <div className="vm-page">
      <AppHeader
        title="Recordings"
        description="Capture your voice. Hear the difference."
        action={
          <Button
            variant={status.recording ? "danger" : "primary"}
            disabled={runtime.status.state !== "running" || recording.busy}
            onClick={() => void (status.recording ? recording.stop() : recording.start())}
          >
            {status.recording ? <Square size={16} /> : <Circle size={16} />}
            {status.recording ? "Stop recording" : "New recording"}
          </Button>
        }
      />
      <div className="vm-toolbar">
        <SearchInput label="Search current recording" value={query} onChange={setQuery} />
        <Badge>{status.hasRecording ? "1 session" : "No sessions"}</Badge>
      </div>
      <Card>
        <div className="vm-section-heading">
          <h2>Session recordings</h2>
          <Badge tone={status.recording ? "warning" : "neutral"}>
            {status.recording ? "Recording" : "Local storage"}
          </Badge>
        </div>
        {status.hasRecording && "current session".includes(query.toLowerCase()) ? (
          <div className="vm-recording-row">
            <span className="vm-empty-icon">
              <FileAudio size={25} />
            </span>
            <div>
              <strong>Current session</strong>
              <p className="vm-hint">
                {status.durationSeconds.toFixed(1)} seconds · Original & processed audio
              </p>
            </div>
            <div className="vm-actions">
              <Button
                disabled={recording.busy || status.recording}
                onClick={() =>
                  void (status.playing ? recording.stopPlayback() : recording.play("processed"))
                }
              >
                {status.playing ? <Square size={16} /> : <Play size={16} />}
                {status.playing ? "Stop" : "Play"}
              </Button>
              <Button
                disabled={recording.busy || status.recording}
                onClick={() => void recording.exportWav("processed")}
              >
                <Download size={16} />
                Export
              </Button>
              <Button
                variant="ghost"
                aria-label="Delete recording"
                disabled={recording.busy || status.recording}
                onClick={() => setConfirmDelete(true)}
              >
                <Trash2 size={16} />
              </Button>
            </div>
          </div>
        ) : (
          <EmptyState
            title={query ? "No matching recording" : "Your next take starts here"}
            description={
              query
                ? "Try searching for current session."
                : "Start monitoring, then record to capture aligned original and processed audio. Export the takes you want to keep."
            }
          />
        )}
        <p className="vm-hint">
          One current session is retained. Export your original and processed takes as WAV files to
          keep them before starting a new recording.
        </p>
      </Card>
      <RecordingPanel
        status={status}
        busy={recording.busy}
        error={recording.error}
        sessionActive={runtime.status.state === "running"}
        onStart={() => void recording.start()}
        onStop={() => void recording.stop()}
        onPlay={(s) => void recording.play(s)}
        onStopPlayback={() => void recording.stopPlayback()}
        onHear={(e) => void recording.setHear(e)}
        onExport={(s) => void recording.exportWav(s)}
        onDelete={() => setConfirmDelete(true)}
      />
      <StudioDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title="Delete current recording?"
      >
        <p className="vm-hint">
          This removes the current session from the recorder. Export a WAV first if you want to keep
          it.
        </p>
        <Button
          variant="danger"
          onClick={() => {
            void recording.remove();
            setConfirmDelete(false);
          }}
        >
          Delete recording
        </Button>
      </StudioDialog>
    </div>
  );
}
