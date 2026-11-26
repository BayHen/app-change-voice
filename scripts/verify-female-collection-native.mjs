// Actual production WebView2/native bridge. Only authorized owner WAV and muted live input.
import { chromium, expect } from "@playwright/test";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";
const base = "docs/screenshots/female-collection";
await mkdir(base, { recursive: true });
const exe = "src-tauri/target/x86_64-pc-windows-msvc/release/voicemorph-studio.exe";
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const executableSha256 = hash(await readFile(exe));
expect(hash(await readFile(".ai-runtime-build/female-native-release/voicemorph-studio.exe"))).toBe(
  executableSha256,
);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9228");
let invoke, page, original, libraryPreferences;
try {
  page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().includes("tauri.localhost"));
  if (!page) throw Error("Launch the owned production female-collection executable first");
  page.setDefaultTimeout(25000);
  invoke = (command, args = {}) =>
    page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), {
      command,
      args,
    });
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  expect((await invoke("get_ai_engine_status")).mode).toBe("classicDsp");
  original = {
    volume: await invoke("get_volume_settings"),
    noise: await invoke("get_noise_settings"),
    effects: await invoke("get_effect_parameters"),
    ai: (await invoke("get_ai_engine_status")).config,
    active: await invoke("get_active_preset_id"),
  };
  libraryPreferences = await page.evaluate(() =>
    localStorage.getItem("voicemorph-female-library-v1"),
  );
  await invoke("set_volume_settings", {
    settings: { ...original.volume, inputMuted: true, outputMuted: true },
  });
  const all = await invoke("get_all_presets"),
    female = all.filter((p) => p.category === "Female");
  expect(female).toHaveLength(20);
  if (!(await page.locator(".vm-female-library").count()))
    await page.getByRole("button", { name: "Female collection", exact: true }).click();
  await expect(page.getByRole("heading", { name: "21 voices", exact: true })).toBeVisible();
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  await expect(
    page.getByRole("button", { name: "Play voice preview", exact: true }),
  ).toBeDisabled();
  await page.screenshot({ path: `${base}/collection.png` });
  const favorite = page.getByRole("button", { name: "Favorite Natural Female", exact: true });
  const wasFavorite = await favorite.getAttribute("aria-pressed");
  await favorite.click();
  await expect(favorite).toHaveAttribute("aria-pressed", wasFavorite === "true" ? "false" : "true");
  await page.getByLabel("Search female voices").fill("Vietnamese Female");
  await expect(page.getByRole("heading", { name: "1 voices", exact: true })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Select Vietnamese Female", exact: true }),
  ).toBeVisible();
  await page.getByLabel("Search female voices").fill("");
  const applied = [];
  for (const preset of female) {
    const result = await invoke("apply_preset", { id: preset.id });
    expect(result.parameters.pitchSemitones).toBeCloseTo(preset.pitchSemitones, 4);
    expect(result.parameters.formantSemitones).toBeCloseTo(preset.formantSemitones, 4);
    expect(result.parameters.deEssStrength).toBeCloseTo(preset.parameters.deEssStrength, 4);
    applied.push({ id: preset.id, parameters: result.parameters });
  }
  await page.getByRole("button", { name: "Select Soft Female", exact: true }).click();
  await page.getByRole("button", { name: "Use DSP voice", exact: true }).click();
  await expect
    .poll(async () => (await invoke("get_effect_parameters")).pitchSemitones)
    .toBeCloseTo(1.4, 3);
  await expect(page.getByLabel("Female pitch")).toHaveValue("1.4");
  await expect(page.getByLabel("Female pitch")).toBeEnabled();
  await page.getByLabel("Female pitch").press("ArrowRight");
  await expect
    .poll(async () => (await invoke("get_effect_parameters")).pitchSemitones)
    .toBeCloseTo(1.5, 3);
  await page.getByLabel("Female formant").press("ArrowRight");
  await expect
    .poll(async () => (await invoke("get_effect_parameters")).formantSemitones)
    .toBeCloseTo(1.1, 3);
  expect((await invoke("get_effect_parameters")).pitchSemitones).toBeCloseTo(1.5, 3);
  await page
    .locator(".vm-female-library")
    .getByRole("button", { name: "Voice Lab", exact: true })
    .click();
  await expect(page.getByLabel("Pitch", { exact: true })).toHaveValue("1.5");
  await page.getByRole("button", { name: "Female collection", exact: true }).click();
  const sourcePath = path.resolve(".ai-runtime-build/assets/owner-sample.wav");
  let refused = false;
  try {
    await invoke("render_voice_preview", {
      presetId: female[0].id,
      sourcePath,
      consentConfirmed: false,
    });
  } catch {
    refused = true;
  }
  expect(refused).toBe(true);
  const previews = [];
  for (const preset of [female[0], female[1], female[19]]) {
    const result = await invoke("render_voice_preview", {
      presetId: preset.id,
      sourcePath,
      consentConfirmed: true,
    });
    expect(result.peak).toBeGreaterThan(0);
    expect(result.fullScaleSamples).toBe(0);
    expect(result.wav.length).toBeGreaterThan(100000);
    const { wav, ...stats } = result;
    previews.push({ id: preset.id, ...stats, wavSha256: hash(new Uint8Array(wav)) });
  }
  const beforeAI = await invoke("get_audio_runtime_status");
  expect(beforeAI.state).toBe("stopped");
  await invoke("set_ai_engine_config", {
    config: {
      ...original.ai,
      provider: "cuda",
      allowCpuFallback: false,
      chunkFrames: 8192,
      crossfadeFrames: 1024,
      extraContextFrames: 4096,
    },
  });
  const aiPreview = await invoke("render_ai_voice_preview", { sourcePath, consentConfirmed: true });
  expect(aiPreview.peak).toBeGreaterThan(0.0001);
  expect(aiPreview.fullScaleSamples).toBe(0);
  const { wav: aiWav, ...aiPreviewStats } = aiPreview;
  await writeFile(
    ".ai-runtime-build/female-collection/ai-native-preview.wav",
    new Uint8Array(aiWav),
  );
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  await page.getByRole("button", { name: "AI", exact: true }).click();
  await page.getByRole("button", { name: "Select Tsukuyomi-chan · Normal 1", exact: true }).click();
  await page.screenshot({ path: `${base}/ai-file-verified.png` });
  await page.getByRole("button", { name: "Use AI voice", exact: true }).click();
  await expect
    .poll(async () => (await invoke("get_ai_engine_status")).state, {
      timeout: 180000,
      intervals: [1000, 2000],
    })
    .toBe("ready");
  const ready = await invoke("get_ai_engine_status");
  expect(ready.provider).toBe("cuda");
  await page.screenshot({ path: `${base}/ai-ready.png` });
  await invoke("stop_ai_conversion");
  await invoke("select_classic_voice");
  await invoke("apply_preset", { id: female[0].id });
  await invoke("set_volume_settings", {
    settings: { ...original.volume, inputMuted: true, outputMuted: true },
  });
  await page.getByRole("switch", { name: "Monitor", exact: true }).click();
  await expect.poll(async () => (await invoke("get_audio_runtime_status")).state).toBe("running");
  await invoke("start_virtual_routing");
  const live = await invoke("get_audio_runtime_status"),
    route = await invoke("get_routing_status");
  expect(route.state).toBe("active");
  await invoke("stop_virtual_routing");
  await invoke("stop_monitoring");
  const layouts = [];
  for (const [width, height] of [
    [1280, 800],
    [900, 650],
    [1600, 900],
  ]) {
    await page.setViewportSize({ width, height });
    await page.getByRole("button", { name: "All", exact: true }).click();
    await page.evaluate(() => scrollTo(0, 0));
    const metrics = await page.evaluate(() => ({
      width: innerWidth,
      scrollWidth: document.documentElement.scrollWidth,
      statusbar: !!document.querySelector(".vm-statusbar"),
    }));
    expect(metrics.scrollWidth).toBeLessThanOrEqual(width);
    expect(metrics.statusbar).toBe(true);
    layouts.push(metrics);
    await page.screenshot({ path: `${base}/collection-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 800 });
  await page
    .locator(".vm-female-library")
    .getByRole("button", { name: "Voice Lab", exact: true })
    .click();
  const workspaces = [];
  for (const label of ["Voice Lab", "Soundboard", "AI Voice Studio", "Recordings", "Settings"]) {
    await page
      .getByRole("navigation", { name: "Primary navigation", exact: true })
      .getByRole("button", { name: label, exact: true })
      .click();
    await expect(page.getByRole("heading", { name: label, exact: true })).toBeVisible();
    await expect(
      page.getByRole("contentinfo", { name: "Audio status", exact: true }),
    ).toBeVisible();
    await page.evaluate(() => scrollTo(0, 0));
    await page.screenshot({
      path: `${base}/workspace-${label.toLowerCase().replaceAll(" ", "-")}.png`,
    });
    workspaces.push(label);
  }
  await writeFile(
    `${base}/native-report.json`,
    JSON.stringify(
      {
        timestampUtc: new Date().toISOString(),
        executableSha256,
        applied,
        previews,
        refusedWithoutConsent: refused,
        aiPreview: { ...aiPreviewStats, wavSha256: hash(new Uint8Array(aiWav)) },
        ready,
        live,
        route,
        layouts,
        workspaces,
        scope:
          "Real production bridge; licensed Japanese owner speech; no TikTok audio, no microphone recording, muted monitor/routing test. Preview WAVs not played by this automated test. Not a listening evaluation, Vietnamese test, actual male-input test, Discord/OBS/Zoom call or hardware latency measurement.",
      },
      null,
      2,
    ),
  );
  console.log("Production female collection, DSP/AI file previews and muted routing verified");
} finally {
  if (invoke && original) {
    await invoke("stop_ai_conversion").catch(() => {});
    await invoke("select_classic_voice").catch(() => {});
    await invoke("stop_virtual_routing").catch(() => {});
    await invoke("stop_monitoring").catch(() => {});
    if (original.active) await invoke("apply_preset", { id: original.active }).catch(() => {});
    await invoke("set_effect_parameters", { parameters: original.effects });
    await invoke("set_noise_settings", { settings: original.noise });
    await invoke("set_volume_settings", { settings: original.volume });
    await invoke("set_ai_engine_config", { config: original.ai });
    await page.evaluate((value) => {
      if (value === null) localStorage.removeItem("voicemorph-female-library-v1");
      else localStorage.setItem("voicemorph-female-library-v1", value);
    }, libraryPreferences);
  }
  await browser.close();
}
