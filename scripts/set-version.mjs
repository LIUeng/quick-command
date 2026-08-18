import { readFileSync, writeFileSync } from "node:fs";

const nextVersion = process.argv.slice(2).find((argument) => argument !== "--");
const stableSemver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

if (!nextVersion || !stableSemver.test(nextVersion)) {
  console.error("Usage: pnpm version:set <major.minor.patch>");
  console.error("Example: pnpm version:set 0.3.0");
  process.exit(1);
}

const targets = [
  {
    label: "package.json",
    url: new URL("../package.json", import.meta.url),
    pattern: /(\"version\"\s*:\s*\")[^\"]+(\")/,
  },
  {
    label: "src-tauri/Cargo.toml",
    url: new URL("../src-tauri/Cargo.toml", import.meta.url),
    pattern: /(^version\s*=\s*\")[^\"]+(\")/m,
  },
  {
    label: "src-tauri/tauri.conf.json",
    url: new URL("../src-tauri/tauri.conf.json", import.meta.url),
    pattern: /(\"version\"\s*:\s*\")[^\"]+(\")/,
  },
  {
    label: "src-tauri/Cargo.lock",
    url: new URL("../src-tauri/Cargo.lock", import.meta.url),
    pattern: /(\[\[package\]\]\nname = \"quick-command\"\nversion = \")[^\"]+(\")/,
  },
];

const updates = targets.map((target) => {
  const current = readFileSync(target.url, "utf8");
  if (!target.pattern.test(current)) {
    throw new Error(`Cannot locate the version field in ${target.label}; no files were changed.`);
  }
  return {
    ...target,
    current,
    next: current.replace(target.pattern, `$1${nextVersion}$2`),
  };
});

for (const update of updates) {
  if (update.current !== update.next) writeFileSync(update.url, update.next);
}

console.log(`Quick Command version synchronized to ${nextVersion}:`);
for (const target of targets) console.log(`- ${target.label}`);
console.log("Run `pnpm version:check` before creating the release commit and tag.");
