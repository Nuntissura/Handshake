import fs from "node:fs";
import path from "node:path";

function parseVersion(name) {
  const match = name.match(/_v(\d+(?:\.\d+)*)\.md$/);
  if (!match) return null;
  return match[1].split(".").map((part) => Number(part));
}

function compareVersions(a, b) {
  const maxLen = Math.max(a.length, b.length);
  for (let i = 0; i < maxLen; i += 1) {
    const left = a[i] ?? 0;
    const right = b[i] ?? 0;
    if (left !== right) return left - right;
  }
  return 0;
}

const repoRoot = process.cwd();
const specFiles = fs
  .readdirSync(repoRoot)
  .filter((name) => name.startsWith("Handshake_Master_Spec_v") && name.endsWith(".md"));

if (specFiles.length === 0) {
  console.error("No Handshake_Master_Spec_v*.md files found in repo root.");
  process.exit(1);
}

const parsed = specFiles
  .map((name) => ({ name, version: parseVersion(name) }))
  .filter((item) => Array.isArray(item.version));

if (parsed.length === 0) {
  console.error("Failed to parse spec versions from Handshake_Master_Spec_v*.md.");
  process.exit(1);
}

parsed.sort((a, b) => compareVersions(a.version, b.version));
const latest = parsed[parsed.length - 1].name;

const specCurrentPath = path.join(repoRoot, "docs", "SPEC_CURRENT.md");
if (!fs.existsSync(specCurrentPath)) {
  console.error("docs/SPEC_CURRENT.md not found.");
  process.exit(1);
}

const specCurrent = fs.readFileSync(specCurrentPath, "utf8");
if (!specCurrent.includes(latest)) {
  console.error(`SPEC_CURRENT does not reference latest spec: ${latest}`);
  process.exit(1);
}

console.log(`SPEC_CURRENT ok: ${latest}`);
