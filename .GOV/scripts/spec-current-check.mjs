import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

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

function resolveRepoRoot() {
  try {
    const out = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    })
      .trim();
    if (out) return out;
  } catch {
    // ignore (e.g., running outside a git checkout)
  }
  return process.cwd();
}

const repoRoot = resolveRepoRoot();
const specFiles = fs
  .readdirSync(repoRoot)
  .filter((name) => name.startsWith("Handshake_Master_Spec_v") && name.endsWith(".md"));

if (specFiles.length === 0) {
  console.error(`No Handshake_Master_Spec_v*.md files found in repo root: ${repoRoot}`);
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

const specCurrentCanonicalPath = path.join(repoRoot, ".GOV", "roles_shared", "SPEC_CURRENT.md");
if (!fs.existsSync(specCurrentCanonicalPath)) {
  console.error(".GOV/roles_shared/SPEC_CURRENT.md not found.");
  process.exit(1);
}

const specCurrentCanonical = fs.readFileSync(specCurrentCanonicalPath, "utf8");
if (!specCurrentCanonical.includes(latest)) {
  console.error(`.GOV/roles_shared/SPEC_CURRENT.md does not reference latest spec: ${latest}`);
  process.exit(1);
}

console.log(`SPEC_CURRENT ok: ${latest}`);
