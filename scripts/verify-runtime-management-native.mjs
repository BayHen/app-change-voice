// Actual production WebView2/command bridge. Never starts an audio device.
import { chromium, expect } from "@playwright/test";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";

const base = "docs/screenshots/runtime-management/native";
await mkdir(base, { recursive: true });
const digest = JSON.parse(
  await readFile(".ai-runtime-build/bootstrap-installed/current.json", "utf8"),
);
const source = path.resolve(".ai-runtime-build/bootstrap-installed/releases", digest);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const expectedSha256 = sha256(await readFile(path.join(source, "runtime-manifest.json")));
const browser = await chromium.connectOverCDP("http://127.0.0.1:9224");
try {
  const page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().includes("tauri.localhost"));
  if (!page) throw new Error("Launch the production native test executable first");
  page.setDefaultTimeout(15000);
  const invoke = (command, args = {}) =>
    page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), {
      command,
      args,
    });
  const audioBefore = await invoke("get_audio_runtime_status");
  expect(audioBefore.state).toBe("stopped");
  const initial = await invoke("get_ai_engine_status");
  expect(initial.mode).toBe("classicDsp");
  await page
    .getByRole("navigation", { name: "Primary navigation", exact: true })
    .getByRole("button", { name: "AI Voice Studio", exact: true })
    .click();
  let installed = initial;
  if (!initial.runtimeInstalled) {
    const installing = invoke("install_ai_runtime", {
      source,
      expectedSha256,
      publisherConfirmed: true,
    });
    await expect
      .poll(async () => (await invoke("get_ai_engine_status")).install.installing, {
        timeout: 30000,
      })
      .toBe(true);
    await page.screenshot({ path: `${base}/runtime-verification.png`, fullPage: true });
    installed = await installing;
  }
  expect(installed.runtimeInstalled).toBe(true);
  expect(installed.mode).toBe("classicDsp");
  console.log("Verified portable runtime installed through actual native command");
  const hardware = await invoke("detect_ai_hardware");
  expect(hardware.hardware.torchCudaAvailable).toBe(true);
  expect(hardware.hardware.providers).toContain("CUDAExecutionProvider");
  await page.screenshot({ path: `${base}/runtime-hardware.png`, fullPage: true });
  const modelsBefore = await invoke("get_ai_models");
  if (!modelsBefore.some((m) => m.id === "tsukuyomi-official-normal1")) {
    await page.getByRole("button", { name: "Get official Tsukuyomi-chan voice" }).click();
    const dialog = page.getByRole("dialog", { name: "Tsukuyomi-chan official voice" });
    const prepare = dialog.getByRole("button", { name: "Download & prepare official voice" });
    await expect(prepare).toBeDisabled();
    await page.screenshot({ path: `${base}/owner-model-terms.png`, fullPage: true });
    await dialog.getByRole("checkbox").check();
    await prepare.click();
    await expect
      .poll(async () => (await invoke("get_ai_engine_status")).install.installing, {
        timeout: 15000,
      })
      .toBe(true);
    await page.screenshot({ path: `${base}/owner-model-progress.png`, fullPage: true });
    await expect
      .poll(
        async () =>
          (await invoke("get_ai_models")).some((m) => m.id === "tsukuyomi-official-normal1"),
        { timeout: 300000, intervals: [1000, 2000, 3000] },
      )
      .toBe(true);
  }
  await expect
    .poll(async () => (await invoke("get_ai_engine_status")).install.installing, { timeout: 30000 })
    .toBe(false);
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  expect((await invoke("get_ai_engine_status")).mode).toBe("classicDsp");
  await page
    .getByLabel("Selected AI voice", { exact: true })
    .selectOption("tsukuyomi-official-normal1");
  await page.getByRole("button", { name: "Load & start AI", exact: true }).click();
  await expect
    .poll(async () => (await invoke("get_ai_engine_status")).state, {
      timeout: 180000,
      intervals: [1000, 2000],
    })
    .toBe("ready");
  const ready = await invoke("get_ai_engine_status");
  expect(ready.provider).toBe("cuda");
  expect(ready.completedChunks).toBe(0);
  expect(ready.roundTripMs).toBeNull();
  await expect(page.getByText("AI model ready. Waiting for audio.", { exact: true })).toBeVisible();
  await page.screenshot({ path: `${base}/ai-ready-cuda-full.png`, fullPage: true });
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: `${base}/ai-ready-cuda.png` });
  const stopped = await invoke("stop_ai_conversion");
  expect(stopped.mode).toBe("wOkada");
  const restored = await invoke("select_classic_voice");
  expect(restored.mode).toBe("classicDsp");
  const audioAfter = await invoke("get_audio_runtime_status");
  expect(audioAfter.state).toBe("stopped");
  const virtualOutputs = await invoke("get_virtual_output_devices");
  const executableSha256 = sha256(await readFile("src-tauri/target/release/voicemorph-studio.exe"));
  expect(
    sha256(await readFile(".ai-runtime-build/native-runtime-check/voicemorph-studio.exe")),
  ).toBe(executableSha256);
  await writeFile(
    `${base}/report.json`,
    JSON.stringify(
      {
        timestampUtc: new Date().toISOString(),
        environment:
          "Production Windows Tauri WebView2; real runtime, owner model and command bridge; audio devices never started",
        executableSha256,
        source,
        expectedSha256,
        initial,
        audioBefore,
        installed,
        hardware,
        models: await invoke("get_ai_models"),
        ready,
        stopped,
        restored,
        audioAfter,
        virtualOutputs,
        limits:
          "File-based inference is verified separately. This UI check does not claim hardware loopback, microphone quality or conferencing compatibility.",
      },
      null,
      2,
    ),
  );
  console.log(
    "Native runtime/model installation, CUDA readiness, explicit stop/Classic restore passed; microphone stayed stopped.",
  );
} finally {
  await browser.close();
}
