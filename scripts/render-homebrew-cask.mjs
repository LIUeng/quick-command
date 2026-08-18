import { dirname } from "node:path";
import { mkdirSync, writeFileSync } from "node:fs";

const options = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  const name = process.argv[index];
  const value = process.argv[index + 1];
  if (!name?.startsWith("--") || !value) {
    console.error("Usage: node scripts/render-homebrew-cask.mjs --version <version> --url <dmg-url> --sha256 <sha256> --output <path>");
    process.exit(1);
  }
  options.set(name.slice(2), value);
}

const version = options.get("version");
const url = options.get("url");
const sha256 = options.get("sha256");
const output = options.get("output");

if (!version || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
  throw new Error("Homebrew Cask version must use stable major.minor.patch SemVer.");
}
if (!url || !url.startsWith(`https://github.com/LIUeng/quick-command/releases/download/v${version}/`) || !url.endsWith(".dmg")) {
  throw new Error("Homebrew Cask URL must reference the matching Quick Command GitHub Release DMG.");
}
if (!sha256 || !/^[a-f0-9]{64}$/.test(sha256)) {
  throw new Error("Homebrew Cask SHA256 must contain exactly 64 lowercase hexadecimal characters.");
}
if (!output) throw new Error("Homebrew Cask output path is required.");

const cask = `cask "quick-command" do
  version "${version}"
  sha256 "${sha256}"

  url "${url}",
      verified: "github.com/LIUeng/quick-command/"
  name "Quick Command"
  desc "Fast macOS launcher for local developer workflows"
  homepage "https://github.com/LIUeng/quick-command"

  auto_updates true

  app "Quick Command.app"

  zap trash: [
    "~/Library/Application Support/com.quickcommand.launcher",
    "~/Library/Caches/com.quickcommand.launcher",
    "~/Library/Preferences/com.quickcommand.launcher.plist",
  ]
end
`;

mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, cask);
console.log(`Rendered Quick Command ${version} Cask at ${output}.`);
