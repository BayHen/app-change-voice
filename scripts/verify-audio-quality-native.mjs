// Production native bridge; synthetic/muted audio only. No raw microphone WAV.
import { chromium, expect } from "@playwright/test";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";
const base = "docs/screenshots/audio-quality";
await mkdir(base, { recursive: true });
const executableSha256 = createHash("sha256")
  .update(await readFile("src-tauri/target/x86_64-pc-windows-msvc/release/voicemorph-studio.exe"))
  .digest("hex");
expect(
  createHash("sha256")
    .update(await readFile(".ai-runtime-build/audio-quality-final-current/voicemorph-studio.exe"))
    .digest("hex"),
).toBe(executableSha256);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9224");
let invoke, originalVolume, originalQuality;
try {
  const page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().includes("tauri.localhost"));
  if (!page) throw new Error("Launch the owned production quality-test executable first");
  page.setDefaultTimeout(20000);
  invoke = (command, args = {}) =>
    page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), {
      command,
      args,
    });
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  originalVolume = await invoke("get_volume_settings");
  originalQuality = await invoke("get_audio_quality");
  expect((await invoke("get_ai_engine_status")).mode).toBe("classicDsp");
  await invoke("set_volume_settings", {
    settings: { ...originalVolume, inputMuted: true, outputMuted: true },
  });
  await page
    .getByRole("navigation", { name: "Primary navigation", exact: true })
    .getByRole("button", { name: "Settings", exact: true })
    .click();
  await page.getByRole("button", { name: "Audio Quality Lab", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Audio Quality Lab", exact: true })).toBeVisible();
  const record = page.getByRole("button", {
    name: "Record selected diagnostic stages",
    exact: true,
  });
  await expect(record).toBeDisabled();
  expect((await invoke("get_stage_capture_status")).running).toBe(false);
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: `${base}/stopped.png`, fullPage: true });
  await page.screenshot({ path: `${base}/stopped-viewport.png` });
  await page.getByRole("switch", { name: "Monitor", exact: true }).click();
  await expect.poll(async () => (await invoke("get_audio_runtime_status")).state).toBe("running");
  let refused = false;
  try {
    await invoke("start_stage_capture", { stages: ["mixer"], seconds: 1, consentConfirmed: false });
  } catch {
    refused = true;
  }
  expect(refused).toBe(true);
  await invoke("start_virtual_routing");
  // Defaults select mixer and virtual output; input is muted before the stream starts.
  await page.getByLabel("Diagnostic capture seconds").fill("3");
  await page
    .getByRole("checkbox", {
      name: "I explicitly allow recording these audio stages to local WAV files.",
    })
    .check();
  await record.click();
  await expect(page.getByText("Local diagnostic recording", { exact: true })).toBeVisible();
  await page.screenshot({ path: `${base}/recording-local.png`, fullPage: true });
  await expect
    .poll(async () => (await invoke("get_stage_capture_status")).running, { timeout: 15000 })
    .toBe(false);
  const capture = await invoke("get_stage_capture_status");
  expect(capture.incomplete).toBe(false);
  expect(capture.files.map((f) => f.stage).sort()).toEqual(["mixer", "virtualOutput"]);
  expect(capture.files.every((f) => f.frames > 0 && f.peak === 0)).toBe(true);
  const live = await invoke("get_audio_runtime_status");
  expect(live.inputSampleFormat).toBeTruthy();
  await expect(page.getByText(/Local capture finalized;/)).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: `${base}/measured.png`, fullPage: true });
  await page.screenshot({ path: `${base}/measured-viewport.png` });
  const viewport = await page.evaluate(() => ({ width: innerWidth, height: innerHeight }));
  await page.setViewportSize({ width: 900, height: 650 });
  await page.screenshot({ path: `${base}/compact-900.png` });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.setViewportSize(viewport);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await invoke("set_audio_quality", { settings: { profile: "highQuality" } });
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  await invoke("set_audio_quality", { settings: originalQuality });
  const digest = JSON.parse(
    await readFile(".ai-runtime-build/bootstrap-installed/current.json", "utf8"),
  );
  const source = path.resolve(".ai-runtime-build/bootstrap-installed/releases", digest);
  const expectedSha256 = createHash("sha256")
    .update(await readFile(path.join(source, "runtime-manifest.json")))
    .digest("hex");
  const manifest = JSON.parse(await readFile(path.join(source, "runtime-manifest.json"), "utf8"));
  const current = await invoke("get_ai_engine_status");
  const installed =
    current.runtimeInstalled && current.runtimeVersion === manifest.version
      ? current
      : await invoke("install_ai_runtime", { source, expectedSha256, publisherConfirmed: true });
  expect(installed.runtimeInstalled).toBe(true);
  const hardware = await invoke("detect_ai_hardware");
  expect(hardware.hardware.torchCudaAvailable).toBe(true);
  await invoke("start_ai_conversion", { modelId: "tsukuyomi-official-normal1" });
  await expect
    .poll(async () => (await invoke("get_ai_engine_status")).state, {
      timeout: 180000,
      intervals: [1000, 2000],
    })
    .toBe("ready");
  const ready = await invoke("get_ai_engine_status");
  expect(ready.provider).toBe("cuda");
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  await invoke("stop_ai_conversion");
  await invoke("select_classic_voice");
  await writeFile(
    `${base}/native-report.json`,
    JSON.stringify(
      {
        timestampUtc: new Date().toISOString(),
        executableSha256,
        refusedWithoutConsent: refused,
        capture,
        live,
        installed,
        hardware,
        ready,
        source,
        expectedSha256,
        scope:
          "Production WebView2/native commands; muted input/output; only silent mixer/virtual WAVs; no raw microphone recording or Discord call.",
      },
      null,
      2,
    ),
  );
  console.log("Production Audio Quality Lab and guarded runtime verified");
} finally {
  if (invoke) {
    await invoke("stop_ai_conversion").catch(() => {});
    await invoke("select_classic_voice").catch(() => {});
    await invoke("stop_virtual_routing").catch(() => {});
    await invoke("stop_monitoring").catch(() => {});
    if (originalVolume) await invoke("set_volume_settings", { settings: originalVolume });
    if (originalQuality) await invoke("set_audio_quality", { settings: originalQuality });
  }
  await browser.close();
}
