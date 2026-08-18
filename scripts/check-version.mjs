import { readFileSync } from "node:fs";

const packageVersion = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")).version;
const tauriVersion = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8")).version;
const cargoContents = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const cargoVersion = cargoContents.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const expectedVersion = process.argv.slice(2).find((argument) => argument !== "--");

const versions = { packageVersion, tauriVersion, cargoVersion };
const values = Object.values(versions);
if (!cargoVersion || new Set(values).size !== 1) {
  console.error("Version mismatch:", versions);
  process.exit(1);
}

if (expectedVersion && packageVersion !== expectedVersion) {
  console.error(`Release tag expects ${expectedVersion}, but project version is ${packageVersion}.`);
  process.exit(1);
}

console.log(`Quick Command version ${packageVersion} is synchronized.`);
