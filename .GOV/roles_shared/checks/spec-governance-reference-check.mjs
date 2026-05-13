import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";
import { findLatestMasterSpecAtRepo, resolveSpecCurrentAtRepo } from "../scripts/lib/spec-current-lib.mjs";

function resolveRepoRoot() {
  const fileRelativeRepoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  try {
    const out = execFileSync("git", ["-C", fileRelativeRepoRoot, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore and fall back.
  }
  return fileRelativeRepoRoot;
}

function readRequired(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Missing required file: ${filePath.replace(/\\/g, "/")}`);
  }
  return fs.readFileSync(filePath, "utf8");
}

const repoRoot = resolveRepoRoot();
process.chdir(repoRoot);

const latestSpec = findLatestMasterSpecAtRepo(repoRoot).name;
const resolvedSpec = resolveSpecCurrentAtRepo(repoRoot, { allowLegacy: false });
const errors = [];

const specCurrentPath = path.join(GOV_ROOT_REPO_REL, "spec", "SPEC_CURRENT.md");
const buildOrderPath = path.join(GOV_ROOT_REPO_REL, "roles_shared", "records", "BUILD_ORDER.md");
const pastWorkIndexPath = path.join(GOV_ROOT_REPO_REL, "reference", "PAST_WORK_INDEX.md");

const specCurrent = readRequired(specCurrentPath);
const buildOrder = readRequired(buildOrderPath);
const pastWorkIndex = readRequired(pastWorkIndexPath);

if (resolvedSpec.sourceBaselineFileName !== latestSpec) {
  errors.push(`${specCurrentPath.replace(/\\/g, "/")} source baseline must be ${latestSpec}`);
}
if (!new RegExp(`^-\\s*SPEC_TARGET:\\s*${resolvedSpec.specTargetLabel.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*$`, "mi").test(buildOrder)) {
  errors.push(`${buildOrderPath.replace(/\\/g, "/")} must declare SPEC_TARGET: ${resolvedSpec.specTargetLabel}`);
}
if (!pastWorkIndex.includes(latestSpec)) {
  errors.push(`${pastWorkIndexPath.replace(/\\/g, "/")} must reference the current authoritative spec ${latestSpec}`);
}
if (/current governance and operating rules\.\s*$/mi.test(pastWorkIndex) && /Handshake Codex v0\./i.test(pastWorkIndex)) {
  errors.push(`${pastWorkIndexPath.replace(/\\/g, "/")} still labels an old Codex version as current governance`);
}
if (!pastWorkIndex.includes("Handshake_Codex_v1.4.md")) {
  errors.push(`${pastWorkIndexPath.replace(/\\/g, "/")} must reference Handshake_Codex_v1.4.md as the current governance reference`);
}

if (errors.length > 0) {
  for (const error of errors) console.error(`FAIL: ${error}`);
  process.exit(1);
}

console.log(`spec-governance-reference-check ok: ${resolvedSpec.specTargetLabel}`);
