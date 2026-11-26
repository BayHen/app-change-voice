import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
const base = process.env.VOICEMORPH_SCREENSHOT_DIR ?? "docs/screenshots/redesign/after";
const url = process.env.VOICEMORPH_TEST_URL ?? "http://127.0.0.1:1422";
await mkdir(base, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
const pages = [
  ["voice-lab", "Voice Lab"],
  ["soundboard", "Soundboard"],
  ["ai-studio", "AI Voice Studio"],
  ["recordings", "Recordings"],
  ["settings", "Settings"],
];
const report = [];
const errors = [];
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  reducedMotion: "reduce",
});
const page = await context.newPage();
page.on("pageerror", (error) => errors.push(error.message));
await page.goto(url);
await page.waitForTimeout(600);
for (const [name, label] of pages) {
  await page
    .getByRole("navigation", { name: "Primary navigation", exact: true })
    .getByRole("button", { name: label, exact: true })
    .click();
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${base}/${name}.png` });
  await page.screenshot({ path: `${base}/${name}-full.png`, fullPage: true });
}
for (const [width, height] of [
  [900, 600],
  [1280, 720],
  [1366, 768],
  [1440, 900],
  [1600, 900],
  [1920, 1080],
  [2560, 1440],
]) {
  await page.setViewportSize({ width, height });
  for (const [name, label] of pages) {
    await page
      .getByRole("navigation", { name: "Primary navigation", exact: true })
      .getByRole("button", { name: label, exact: true })
      .click();
    await page.waitForTimeout(80);
    const layout = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      width: innerWidth,
      statusbar: !!document.querySelector(".vm-statusbar"),
      header: document.querySelector("h1")?.textContent,
    }));
    if (layout.scrollWidth > width)
      throw new Error(`Horizontal overflow ${name} ${width}: ${layout.scrollWidth}`);
    if (!layout.statusbar || layout.header !== label)
      throw new Error(`Workspace shell mismatch: ${name} at ${width}`);
    report.push({ page: name, width, height, ...layout });
  }
}
for (const scale of [1, 1.25, 1.5]) {
  const dpi = await browser.newContext({
    viewport: { width: 1366, height: 768 },
    deviceScaleFactor: scale,
    reducedMotion: "reduce",
  });
  const tab = await dpi.newPage();
  await tab.goto(url);
  await tab.waitForTimeout(250);
  await tab.screenshot({ path: `${base}/voice-lab-dpr-${scale}.png` });
  await dpi.close();
}
await writeFile(
  `${base}/layout-report.json`,
  JSON.stringify(
    {
      environment:
        "Edge headless, real browser offline bridge; DPR is browser emulation, not Windows scaling",
      report,
      errors,
    },
    null,
    2,
  ),
);
// Timings describe this browser UI on this host, not live DSP or end-to-end audio latency.
await page.setViewportSize({ width: 1440, height: 900 });
const timings = [];
for (let round = 0; round < 3; round++) {
  for (const [name, label] of pages) {
    const start = performance.now();
    await page
      .getByRole("navigation", { name: "Primary navigation", exact: true })
      .getByRole("button", { name: label, exact: true })
      .click();
    await page.getByRole("heading", { name: label, exact: true }).waitFor();
    await page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
    timings.push({ page: name, ms: +(performance.now() - start).toFixed(2) });
  }
}
const cdp = await context.newCDPSession(page);
await cdp.send("Performance.enable");
const metrics = await cdp.send("Performance.getMetrics");
await writeFile(
  `${base}/performance.json`,
  JSON.stringify(
    {
      url,
      environment:
        "Production browser preview, offline Tauri bridge, Edge headless. Automation-to-paint timings include Playwright overhead; no live audio performance claim.",
      timings,
      metrics: metrics.metrics.filter((x) =>
        ["JSHeapUsedSize", "JSHeapTotalSize", "TaskDuration", "ScriptDuration", "Nodes"].includes(
          x.name,
        ),
      ),
    },
    null,
    2,
  ),
);
await page
  .getByRole("navigation", { name: "Primary navigation", exact: true })
  .getByRole("button", { name: "Soundboard", exact: true })
  .click();
const importButton = page.getByRole("button", { name: "Import audio", exact: true });
await importButton.click();
await page.getByRole("dialog", { name: "Add WAV sound" }).waitFor();
for (let i = 0; i < 8; i++) {
  await page.keyboard.press("Tab");
  if (!(await page.evaluate(() => !!document.activeElement?.closest('[role="dialog"]'))))
    throw new Error("Dialog focus escaped");
}
await page.keyboard.press("Escape");
await page.getByRole("dialog").waitFor({ state: "hidden" });
await expect(importButton).toBeFocused();
const { readFile } = await import("node:fs/promises");
const contact = await browser.newPage({
  viewport: { width: 2200, height: 340 },
  deviceScaleFactor: 1,
});
const pictures = await Promise.all(
  pages.map(
    async ([name, label]) =>
      `<figure><figcaption>${label}</figcaption><img src="data:image/png;base64,${(await readFile(`${base}/${name}.png`)).toString("base64")}" /></figure>`,
  ),
);
await contact.setContent(
  `<html><head><style>body{background:#10121f;color:#ddd;font:16px 'Segoe UI',sans-serif;margin:16px;display:flex;gap:12px}figure{margin:0;width:424px}figcaption{margin:0 0 10px}img{width:424px;border:1px solid #41455e}</style></head><body>${pictures.join("")}</body></html>`,
);
await contact.screenshot({ path: `${base}/all-pages.png` });
await browser.close();
if (errors.length) throw new Error(errors.join("\n"));
console.log(
  `Captured five pages; ${report.length} page/viewport checks; DPR 1/1.25/1.5; no page errors.`,
);
