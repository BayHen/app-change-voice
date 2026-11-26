// Connect only to an explicitly launched local WebView2 test instance.
// No models/runtime are mocked and no microphone stream is started.
import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
const endpoint = process.env.VOICEMORPH_CDP_URL ?? "http://127.0.0.1:9224";
const base = "docs/screenshots/ai-engine/native";
await mkdir(base, { recursive: true });
const browser = await chromium.connectOverCDP(endpoint);
try {
  const pages = browser.contexts().flatMap((context) => context.pages());
  const page = pages.find((candidate) => candidate.url().includes("tauri.localhost"));
  if (!page)
    throw new Error("No native Tauri WebView found; launch the production test executable");
  const invoke = (command, args = {}) =>
    page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), {
      command,
      args,
    });
  const initial = await invoke("get_ai_engine_status");
  if (initial.runtimeInstalled || initial.mode !== "classicDsp") {
    throw new Error("This no-runtime smoke requires a clean Classic DSP test instance");
  }
  const beforeAudio = await invoke("get_audio_runtime_status");
  expect(beforeAudio.state).toBe("stopped");
  await expect(
    page.getByRole("navigation", { name: "Primary navigation", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("navigation", { name: "Primary navigation", exact: true })
    .getByRole("button", { name: "AI Voice Studio", exact: true })
    .click();
  await expect(page.getByText("Classic DSP is your current voice.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Load & start AI" })).toBeDisabled();
  await page.screenshot({ path: `${base}/ai-studio-no-runtime.png`, fullPage: true });
  let startError;
  try {
    await invoke("start_ai_conversion", { modelId: "missing-test-model" });
  } catch (error) {
    startError = String(error);
  }
  if (!startError) throw new Error("Missing runtime unexpectedly started AI");
  const failed = await invoke("get_ai_engine_status");
  expect(failed.mode).toBe("wOkada");
  expect(failed.state).toBe("faulted");
  expect(failed.config.failurePolicy).toBe("mute");
  expect(failed.completedChunks).toBe(0);
  expect(failed.roundTripMs).toBeNull();
  await expect(page.getByText("faulted", { exact: true })).toBeVisible({ timeout: 3000 });
  await page.screenshot({ path: `${base}/ai-failure-muted.png`, fullPage: true });
  const stopped = await invoke("stop_ai_conversion");
  expect(stopped.mode).toBe("wOkada");
  const restored = await invoke("select_classic_voice");
  expect(restored.mode).toBe("classicDsp");
  const afterAudio = await invoke("get_audio_runtime_status");
  expect(afterAudio.state).toBe("stopped");
  await writeFile(
    `${base}/report.json`,
    JSON.stringify(
      {
        environment:
          "Production Windows Tauri WebView2, real command bridge, no runtime/model, audio capture never started",
        initial,
        beforeAudio,
        startError,
        failed,
        stopped,
        restored,
        afterAudio,
        verified: [
          "Classic default",
          "No-runtime load disabled",
          "Real start error",
          "AI remains selected/muted on error",
          "No fabricated telemetry",
          "Stop keeps AI selected",
          "Explicit Classic restore",
          "No automatic microphone start",
        ],
        notVerified: [
          "Real RVC inference",
          "Actual speaker/virtual-cable silence",
          "CPU/CUDA performance",
          "Discord/Zoom/OBS",
        ],
      },
      null,
      2,
    ),
  );
  console.log("Native no-runtime smoke passed; microphone remained stopped.");
} finally {
  await browser.close();
}
