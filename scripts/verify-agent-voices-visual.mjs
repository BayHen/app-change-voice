import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
const directory = "docs/screenshots/phase6";
await mkdir(directory, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
const page = await browser.newPage({
  viewport: { width: 1440, height: 960 },
  reducedMotion: "reduce",
});
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
await page.goto("http://127.0.0.1:1431");
await page.getByRole("button", { name: "Agent Voices", exact: true }).waitFor();
await page.evaluate(() => {
  const voices = [
    { id: "wind-assassin", name: "Wind Assassin" },
    { id: "jade-guardian", name: "Jade Guardian" },
    { id: "jett", name: "Jett-inspired" },
  ];
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = {
    invoke: async (command) => {
      if (command === "get_agent_voices")
        return voices.map((voice) => ({
          ...voice,
          state: "notInstalled",
          binding: null,
          message: "Import an authorized original voice pack.",
        }));
      if (command === "get_ai_models") return [];
      if (command === "get_ai_engine_status")
        return { state: "stopped", runtimeInstalled: false, config: { hybridDsp: false } };
      if (command === "get_voice_pack_status") return { phase: "idle" };
      if (command === "get_effect_parameters")
        return {
          pitchSemitones: 0,
          formantSemitones: 0,
          enabled: true,
          bypassed: false,
          makeupGainDb: 0,
          ringModFrequencyHz: 72,
          ringModMix: 0,
          radioHighPassHz: 280,
          radioLowPassHz: 3400,
          radioDrive: 1,
          radioMix: 0,
          tremoloRateHz: 5,
          tremoloDepth: 0,
          bassDb: 0,
          trebleDb: 0,
        };
      throw new Error("Browser visual fixture: native audio unavailable");
    },
  };
});
await page.getByRole("button", { name: "Agent Voices", exact: true }).click();
await expect(page.getByText("Missing Model", { exact: true })).toHaveCount(2);
await expect(page.getByText("Jett-inspired", { exact: true })).toHaveCount(0);
await expect(page.getByRole("alert")).toHaveCount(0);
await page
  .locator(".vm-original-agent-art")
  .first()
  .evaluate(async (element) => {
    const source = getComputedStyle(element).backgroundImage.match(/^url\("(.*)"\)$/)?.[1];
    if (!source) throw new Error("Missing original Agent portrait");
    const image = new Image();
    image.src = source;
    await image.decode();
  });
await expect(page.getByRole("button", { name: "Load neural voice", exact: true })).toBeDisabled();
await page.screenshot({ path: `${directory}/original-profiles.png` });
await page.getByRole("button", { name: "Import model for Jade Guardian", exact: true }).click();
await expect(page.getByRole("button", { name: "Choose voice-pack.json" })).toBeVisible();
await expect(page.getByRole("button", { name: "Install, validate & assign" })).toBeDisabled();
await page.screenshot({ path: `${directory}/original-pack-import.png` });
await page.keyboard.press("Escape");
await page.setViewportSize({ width: 840, height: 660 });
await page.getByRole("heading", { name: "Agent Voices", exact: true }).scrollIntoViewIfNeeded();
await expect(page.getByRole("button", { name: "Import model for Jade Guardian" })).toBeInViewport();
await page.screenshot({ path: `${directory}/original-profiles-compact.png` });
const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
expect(overflow).toBe(false);
expect(errors).toEqual([]);
await writeFile(
  `${directory}/report.json`,
  JSON.stringify(
    {
      fixture:
        "Actual React app; model library and IPC mocked; no inference or audio devices in browser",
      screenshots: 3,
      visibleOriginalProfiles: 2,
      errors,
      overflow,
    },
    null,
    2,
  ),
);
await browser.close();
console.log(
  "Agent Voices visual checks passed: 3 screenshots, two original cards, no legacy card or document overflow",
);
