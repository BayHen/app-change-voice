import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { ArrowClockwise, DownloadSimple, WarningCircle } from "@phosphor-icons/react";
import * as api from "../services/tauri";
import type { AudioDiagnosticsReport } from "../types/audio";

const value = (input: unknown, suffix = "") =>
  input === null || input === undefined ? "—" : `${String(input)}${suffix}`;

export function AudioDiagnostics() {
  const [report, setReport] = useState<AudioDiagnosticsReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  async function refresh() {
    setBusy(true);
    setError("");
    try {
      setReport(await api.getAudioDiagnostics());
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    let active = true;
    void api
      .getAudioDiagnostics()
      .then((next) => {
        if (active) setReport(next);
      })
      .catch((reason) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, []);

  async function exportReport() {
    const path = await save({
      defaultPath: `voicemorph-diagnostics-${Date.now()}.json`,
      filters: [{ name: "Diagnostics report", extensions: ["json"] }],
    });
    if (!path) return;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await api.exportAudioDiagnostics(path);
      setMessage("Diagnostics report exported.");
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  const metrics = report?.runtime.metrics;
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={() => void refresh()}
          className="rounded-lg bg-white/10 px-3 py-2 text-xs disabled:opacity-50"
        >
          <ArrowClockwise className="mr-2 inline" /> Refresh
        </button>
        <button
          type="button"
          disabled={busy || !report}
          onClick={() => void exportReport()}
          className="rounded-lg bg-cyan-300 px-3 py-2 text-xs font-semibold text-cyan-950 disabled:opacity-50"
        >
          <DownloadSimple className="mr-2 inline" /> Export Diagnostics Report
        </button>
      </div>
      {error && (
        <p role="alert" className="rounded-lg bg-rose-300/10 p-3 text-xs text-rose-200">
          <WarningCircle className="mr-2 inline" />
          {error}
        </p>
      )}
      {message && (
        <p role="status" className="rounded-lg bg-emerald-300/10 p-3 text-xs text-emerald-200">
          {message}
        </p>
      )}
      <dl className="grid gap-x-6 gap-y-3 text-xs sm:grid-cols-2">
        <Metric label="Input device" value={value(report?.activeInputDevice)} />
        <Metric label="Output device" value={value(report?.activeOutputDevice)} />
        <Metric label="Monitoring" value={value(report?.runtime.state)} />
        <Metric label="Virtual routing" value={value(report?.routing.state)} />
        <Metric label="Sample rate" value={value(report?.runtime.sampleRate, " Hz")} />
        <Metric
          label="Buffers (input/output)"
          value={
            metrics ? `${metrics.inputBufferFrames}/${metrics.outputBufferFrames} frames` : "—"
          }
        />
        <Metric
          label="Estimated pipeline latency"
          value={metrics ? `${metrics.estimatedLatencyMs.toFixed(2)} ms` : "—"}
        />
        <Metric
          label="DSP algorithmic latency"
          value={
            metrics && report?.runtime.sampleRate
              ? `${((metrics.dspLatencyFrames / report.runtime.sampleRate) * 1000).toFixed(2)} ms`
              : "—"
          }
        />
        <Metric
          label="Processing estimate"
          value={
            report?.processingCpuEstimatePercent == null
              ? "—"
              : `${report.processingCpuEstimatePercent.toFixed(2)}% of one core`
          }
        />
        <Metric
          label="Underruns / overflows"
          value={metrics ? `${metrics.underrunCount} / ${metrics.overflowCount}` : "—"}
        />
        <Metric label="Stream errors" value={value(metrics?.streamErrorCount)} />
        <Metric label="App version" value={value(report?.applicationVersion)} />
      </dl>
      <p className="text-[11px] leading-5 text-zinc-600">
        Estimated latency is not a measured end-to-end loopback result. Export contains metadata and
        counters only—never audio or recordings.
      </p>
    </div>
  );
}

function Metric({ label, value: metricValue }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4 border-b border-white/[.05] pb-2">
      <dt className="text-zinc-600">{label}</dt>
      <dd className="text-right font-mono text-zinc-300">{metricValue}</dd>
    </div>
  );
}
