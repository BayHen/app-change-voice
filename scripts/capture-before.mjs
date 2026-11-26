import { chromium } from "@playwright/test";
const browser = await chromium.launch({ channel: "msedge", headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto("http://127.0.0.1:1420");
await page.waitForTimeout(700);
for (const [name, label] of [
  ["voice-lab", "Dashboard"],
  ["soundboard", "Soundboard"],
  ["ai-studio", "AI Voice"],
  ["recordings", "Dashboard"],
  ["settings", "Settings"],
]) {
  await page
    .getByRole("navigation", { name: "Primary navigation", exact: true })
    .getByRole("button", { name: label, exact: true })
    .click();
  await page.waitForTimeout(300);
  await page.screenshot({ path: `docs/screenshots/redesign/before/${name}.png`, fullPage: true });
}
await browser.close();
