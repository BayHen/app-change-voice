import { readFileSync } from "node:fs";

const packageVersion = JSON.parse(readFileSync("package.json", "utf8")).version;
const tauriVersion = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const driverManifest = JSON.parse(
  readFileSync("drivers/voicemorph-virtual-audio/driver-package.json", "utf8"),
);
const driverVersion = driverManifest.driverVersion;
const driverAppVersion = driverVersion?.split(".").slice(0, 3).join(".");
const versions = { packageVersion, tauriVersion, cargoVersion };

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(packageVersion)) {
  throw new Error(`Version is not valid SemVer: ${packageVersion}`);
}
if (new Set(Object.values(versions)).size !== 1) {
  throw new Error(`Version mismatch: ${JSON.stringify(versions)}`);
}
if (driverAppVersion !== packageVersion) {
  throw new Error(
    `Driver version ${driverVersion} is not aligned with application ${packageVersion}`,
  );
}

const tag = process.env.RELEASE_TAG?.replace(/^v/, "");
if (tag && tag !== packageVersion) {
  throw new Error(`Tag ${tag} does not match application version ${packageVersion}`);
}
console.log(`Version fields are synchronized: app ${packageVersion}, driver ${driverVersion}`);
