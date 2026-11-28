import { useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { Badge, Button, StudioDialog } from "./ui";
import * as api from "../services/tauri";
export function NeuralModelImport({
  onClose,
  onImported,
}: {
  onClose: () => void;
  onImported: (id: string) => Promise<void>;
}) {
  const [path, setPath] = useState(""),
    [manifest, setManifest] = useState(""),
    [info, setInfo] = useState<api.NeuralInspection | null>(null),
    [name, setName] = useState(""),
    [source, setSource] = useState(""),
    [terms, setTerms] = useState(""),
    [attribution, setAttribution] = useState(""),
    [rights, setRights] = useState(false),
    [busy, setBusy] = useState(""),
    [error, setError] = useState("");
  const reportError = (e: unknown) =>
    setError(
      e instanceof Error
        ? e.message
        : typeof e === "string"
          ? e
          : e && typeof e === "object" && "message" in e
            ? String(e.message)
            : "Import failed. Check the model and try again.",
    );
  const operation = useRef(false);
  async function choose(paths?: string[]) {
    if (operation.current) return;
    operation.current = true;
    setError("");
    try {
      const selected =
        paths?.[0] ??
        (await open({
          multiple: false,
          filters: [
            { name: "ONNX model or voice manifest", extensions: ["onnx", "json"] },
            { name: "ONNX model", extensions: ["onnx"] },
            { name: "Voice manifest", extensions: ["json"] },
          ],
        }));
      if (typeof selected !== "string") return;
      setInfo(null);
      setPath("");
      setRights(false);
      setManifest("");
      let model = selected;
      if (/\.json$/i.test(selected)) {
        const file = await open({
          multiple: false,
          filters: [{ name: "Matching ONNX model", extensions: ["onnx"] }],
        });
        if (typeof file !== "string") return;
        setManifest(selected);
        model = file;
      }
      setBusy("Inspecting model structure and SHA-256…");
      const result = await api.inspectVoiceModel(model);
      setInfo(result);
      setPath(model);
      setName(
        model
          .split(/[\\/]/)
          .pop()
          ?.replace(/\.onnx$/i, "") ?? "",
      );
    } catch (e) {
      reportError(e);
    } finally {
      operation.current = false;
      setBusy("");
    }
  }
  const chooseRef = useRef(choose);
  useEffect(() => {
    chooseRef.current = choose;
  });
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === "drop") {
          if (payload.paths.length !== 1) {
            setError("Drop one ONNX or JSON file at a time");
            return;
          }
          void chooseRef.current(payload.paths);
        }
      })
      .then((remove) => {
        if (disposed) remove();
        else unlisten = remove;
      })
      .catch(reportError);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  async function register() {
    if (!info || !rights || operation.current) return;
    operation.current = true;
    setBusy("Verifying and registering neural model…");
    setError("");
    try {
      const result = manifest
        ? await api.addAIModel(manifest, path, rights)
        : await api.registerRawVoiceModel(path, {
            name,
            sourceUrl: source,
            licenseUrl: terms,
            attribution,
            rightsConfirmed: rights,
            expectedSha256: info.sha256,
          });
      await onImported(result.id);
      onClose();
    } catch (e) {
      reportError(e);
    } finally {
      operation.current = false;
      setBusy("");
    }
  }
  return (
    <StudioDialog
      open
      onOpenChange={(v) => {
        if (!v && !busy) onClose();
      }}
      title="Import a licensed neural voice"
    >
      <p className="vm-hint">
        Select or drop an ONNX file directly, or choose an existing JSON manifest. Compatibility is
        specific to the pinned RVC engine; importing a file does not prove naturalness or voice
        rights.
      </p>
      <Button disabled={!!busy} onClick={() => void choose()}>
        Choose ONNX or JSON
      </Button>
      {busy && (
        <div role="status">
          <p>{busy}</p>
          <progress aria-label="Neural model import progress" />
        </div>
      )}
      {error && (
        <p role="alert" className="vm-form-error">
          {error}
        </p>
      )}
      {info && (
        <>
          <p className="vm-hint" style={{ overflowWrap: "anywhere" }}>
            {path}
          </p>
          <Badge tone={info.compatibility === "candidate" ? "success" : "warning"}>
            {info.compatibility === "candidate"
              ? "RVC candidate · runtime test required"
              : "Incompatible"}
          </Badge>
          {info.issues.length > 0 && (
            <div className="vm-notice">
              <ul>
                {info.issues.map((issue) => (
                  <li key={issue}>{issue}</li>
                ))}
              </ul>
              <p>
                Use a compatible export with embedded RVC metadata. Missing architecture or tensor
                contracts cannot be guessed.
              </p>
            </div>
          )}
          <details>
            <summary>Detected metadata and tensors</summary>
            <pre className="vm-hint" style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>
              {JSON.stringify(info.metadata, null, 2)}
            </pre>
            <ul>
              {[...info.inputs, ...info.outputs].map((t) => (
                <li key={t.name}>
                  {t.name}: type {t.elementType} [{t.shape.join(", ")}]
                </li>
              ))}
            </ul>
            <p className="vm-hint" style={{ overflowWrap: "anywhere" }}>
              SHA-256: {info.sha256}
            </p>
          </details>
          {!manifest && (
            <>
              <label className="vm-field">
                Model name
                <input value={name} onChange={(e) => setName(e.target.value)} />
              </label>
              <label className="vm-field">
                Model source URL
                <input
                  type="url"
                  placeholder="https://…"
                  value={source}
                  onChange={(e) => setSource(e.target.value)}
                />
              </label>
              <label className="vm-field">
                License and voice permission URL
                <input
                  type="url"
                  placeholder="https://…"
                  value={terms}
                  onChange={(e) => setTerms(e.target.value)}
                />
              </label>
              <label className="vm-field">
                Voice owner and attribution
                <input value={attribution} onChange={(e) => setAttribution(e.target.value)} />
              </label>
              <p className="vm-hint">
                Custom terms are recorded as your declaration. The app does not infer voice rights
                from public availability.
              </p>
            </>
          )}
          <label className="vm-check">
            <input type="checkbox" checked={rights} onChange={(e) => setRights(e.target.checked)} />
            I have permission to use this voice and accept its model license
          </label>
          <div className="vm-actions mt-5">
            <Button
              variant="primary"
              disabled={
                !!busy ||
                !rights ||
                info.compatibility !== "candidate" ||
                (!manifest &&
                  (!name.trim() ||
                    !source.startsWith("https://") ||
                    !terms.startsWith("https://") ||
                    !attribution.trim()))
              }
              onClick={() => void register()}
            >
              Register neural model
            </Button>
          </div>
        </>
      )}
    </StudioDialog>
  );
}
