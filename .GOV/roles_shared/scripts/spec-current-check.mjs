import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { GOV_ROOT_REPO_REL } from "./lib/runtime-paths.mjs";
import { findLatestMasterSpecAtRepo, resolveSpecCurrentAtRepo } from "./lib/spec-current-lib.mjs";

function resolveRepoRoot() {
  const fileRelativeRepoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  try {
    const out = execFileSync("git", ["-C", fileRelativeRepoRoot, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    })
      .trim();
    if (out) return out;
  } catch {
    // ignore (e.g., running outside a git checkout)
  }
  return fileRelativeRepoRoot;
}

const repoRoot = resolveRepoRoot();

const specCurrentCanonicalPath = path.join(repoRoot, GOV_ROOT_REPO_REL, "spec", "SPEC_CURRENT.md");
if (!fs.existsSync(specCurrentCanonicalPath)) {
  console.error(`${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md not found.`);
  process.exit(1);
}

const latest = findLatestMasterSpecAtRepo(repoRoot);
const resolved = resolveSpecCurrentAtRepo(repoRoot, { allowLegacy: false });
if (resolved.entrypointType !== "indexed_manifest") {
  console.error(`${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md must resolve to an indexed spec manifest.`);
  process.exit(1);
}

if (resolved.sourceBaselineFileName !== latest.name) {
  console.error(`${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md source baseline is not latest spec: ${latest.name}`);
  process.exit(1);
}

console.log(`SPEC_CURRENT ok: ${resolved.specTargetLabel} (source baseline ${latest.name})`);
