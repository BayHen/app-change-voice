import { chromium, expect } from "@playwright/test";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
const base = "docs/screenshots/soundboard-fix";
await mkdir(base, { recursive: true });
const browser = await chromium.connectOverCDP("http://127.0.0.1:9230");
let invoke, original;
try {
  const page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().includes("tauri.localhost"));
  if (!page) throw Error("Launch owned soundboard-test EXE first");
  invoke = (command, args = {}) =>
    page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), {
      command,
      args,
    });
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  original = await invoke("get_volume_settings");
  await invoke("set_volume_settings", {
    settings: { ...original, inputMuted: true, outputMuted: true },
  });
  await page
    .getByRole("navigation", { name: "Primary navigation", exact: true })
    .getByRole("button", { name: "Soundboard", exact: true })
    .click();
  const sounds = await invoke("get_soundboard_sounds");
  const sound = sounds.find((s) => !s.missing);
  if (!sound) throw Error("No existing valid user clip to verify; no library changes made");
  await expect(page.getByRole("button", { name: "Play", exact: true }).first()).toBeDisabled();
  const stoppedError = await page.evaluate(async (id) => {
    try {
      await window.__TAURI_INTERNALS__.invoke("play_sound", { id });
      return null;
    } catch (error) {
      return error;
    }
  }, sound.id);
  expect(stoppedError.message).toContain("Start audio processing");
  await invoke("stop_sound", { id: sound.id });
  await page.screenshot({ path: `${base}/engine-stopped.png` });
  await page.getByRole("button", { name: "Start audio engine", exact: true }).click();
  await expect
    .poll(async () => (await invoke("get_audio_runtime_status")).state, { timeout: 20000 })
    .toBe("running");
  await expect(page.getByRole("button", { name: "Play", exact: true }).first()).toBeEnabled();
  await page.getByRole("button", { name: "Play", exact: true }).first().click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.getByRole("button", { name: "Stop", exact: true }).first().click();
  const live = await invoke("get_audio_runtime_status");
  await page.screenshot({ path: `${base}/engine-running.png` });
  expect(await page.locator("body").innerText()).not.toContain("[object Object]");
  const bytes = await readFile(
    "src-tauri/target/x86_64-pc-windows-msvc/release/voicemorph-studio.exe",
  );
  await writeFile(
    `${base}/report.json`,
    JSON.stringify(
      {
        timestampUtc: new Date().toISOString(),
        executableSha256: createHash("sha256").update(bytes).digest("hex"),
        soundId: sound.id,
        stoppedError,
        live,
        scope:
          "Actual native bridge with existing clip, microphone/output muted; no routing started, no recording or file/library changes, no audible listening assessment.",
      },
      null,
      2,
    ),
  );
  console.log("Soundboard idle guard, explicit start and Play/Stop verified natively");
} finally {
  if (invoke && original) {
    await invoke("stop_monitoring").catch(() => {});
    await invoke("set_volume_settings", { settings: original });
  }
  await browser.close();
}
