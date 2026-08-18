import { readFileSync, writeFileSync } from "node:fs";

const encodedPublicKey = process.env.QUICK_COMMAND_UPDATER_PUBKEY?.trim();
if (!encodedPublicKey) {
  console.error("Missing QUICK_COMMAND_UPDATER_PUBKEY.");
  process.exit(1);
}

if (!/^[A-Za-z0-9+/]+={0,2}$/.test(encodedPublicKey)) {
  console.error("The updater public key must be the unchanged single-line Base64 content of the generated .pub file.");
  process.exit(1);
}

const decodedPublicKey = Buffer.from(encodedPublicKey, "base64").toString("utf8").trim();
const decodedLines = decodedPublicKey.split(/\r?\n/);
if (!decodedLines[0]?.startsWith("untrusted comment:") || decodedLines.length < 2 || !decodedLines[1]) {
  console.error("The updater public key does not decode to a valid minisign public key.");
  process.exit(1);
}

const templateUrl = new URL("../src-tauri/tauri.release.conf.json", import.meta.url);
const defaultOutputUrl = new URL("../src-tauri/tauri.release.generated.conf.json", import.meta.url);
const outputPath = process.env.QUICK_COMMAND_RELEASE_CONFIG_PATH || defaultOutputUrl;
const releaseConfiguration = JSON.parse(readFileSync(templateUrl, "utf8"));

releaseConfiguration.plugins ??= {};
releaseConfiguration.plugins.updater ??= {};
releaseConfiguration.plugins.updater.pubkey = encodedPublicKey;

writeFileSync(outputPath, `${JSON.stringify(releaseConfiguration, null, 2)}\n`, { mode: 0o600 });
console.log("Generated a Tauri release configuration with a validated updater public key.");
