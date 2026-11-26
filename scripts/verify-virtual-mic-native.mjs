// Native production bridge smoke; muted input/output, no recording or call.
import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
const base = "docs/screenshots/virtual-mic-review";
await mkdir(base, { recursive: true });
const browser = await chromium.connectOverCDP("http://127.0.0.1:9224");
let invoke, originalVolume;
try {
  const page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().includes("tauri.localhost"));
  if (!page) throw new Error("Launch the native review executable with loopback CDP first");
  invoke = (command, args = {}) =>
    page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), {
      command,
      args,
    });
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  originalVolume = await invoke("get_volume_settings");
  await invoke("set_volume_settings", {
    settings: { ...originalVolume, inputMuted: true, outputMuted: true },
  });
  const devices = await invoke("get_virtual_output_devices");
  const cable = devices.find((d) => /^CABLE Input \(/.test(d.name));
  const headphones = devices.find((d) => !d.likelyVirtual && /Headphones/.test(d.name));
  if (!cable || !headphones) throw new Error("Required existing cable/headphone endpoints missing");
  const before = await invoke("get_routing_configuration");
  let rejectedPhysical;
  try {
    await invoke("select_virtual_output_device", { id: headphones.id });
  } catch (error) {
    rejectedPhysical = error;
  }
  expect(rejectedPhysical).toBeTruthy();
  expect((await invoke("get_routing_configuration")).outputDeviceId).toBe(before.outputDeviceId);
  await page
    .getByRole("navigation", { name: "Primary navigation", exact: true })
    .getByRole("button", { name: "Settings", exact: true })
    .click();
  await page.getByRole("button", { name: "Virtual microphone", exact: true }).click();
  await expect(page.getByLabel("Virtual cable playback endpoint")).toBeVisible();
  await page.getByLabel("Virtual cable playback endpoint").selectOption(cable.id);
  await expect
    .poll(async () => (await invoke("get_routing_configuration")).outputDeviceId)
    .toBe(cable.id);
  expect((await invoke("get_audio_runtime_status")).state).toBe("stopped");
  await expect(page.getByRole("button", { name: "Start routing", exact: true })).toBeDisabled();
  await page.screenshot({ path: `${base}/cable-selected.png` });
  await page.getByRole("switch", { name: "Monitor", exact: true }).click();
  await expect.poll(async () => (await invoke("get_audio_runtime_status")).state).toBe("running");
  await page.getByRole("button", { name: "Start routing", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop routing", exact: true })).toBeVisible();
  await expect(page.getByText("Virtual mic: live", { exact: true })).toBeVisible();
  const active = await invoke("get_routing_status");
  expect(active.outputDeviceId).toBe(cable.id);
  await page.screenshot({ path: `${base}/routing-active.png` });
  await invoke("stop_virtual_routing");
  await invoke("stop_monitoring");
  await writeFile(
    `${base}/native-report.json`,
    JSON.stringify(
      {
        timestampUtc: new Date().toISOString(),
        originalRoute: before,
        selectedCable: cable,
        rejectedPhysical,
        active,
        stopped: await invoke("get_audio_runtime_status"),
        scope:
          "Real production UI/commands. Input and headphone output muted throughout; no recording/call. Paired-capture signal proof is in loopback-report.json. Correct CABLE Input remains selected; volume restored after test.",
      },
      null,
      2,
    ),
  );
  console.log(
    "Native physical-route rejection, cable selection, Monitor + routing and shared footer state passed.",
  );
} finally {
  if (invoke && originalVolume) {
    await invoke("stop_virtual_routing").catch(() => {});
    await invoke("stop_monitoring").catch(() => {});
    await invoke("set_volume_settings", { settings: originalVolume });
  }
  await browser.close();
}
