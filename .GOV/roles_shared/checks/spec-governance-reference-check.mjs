import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";

function resolveRepoRoot() {
  try {
    const out = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore and fall back.
  }
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function parseVersion(fileName) {
  const match = String(fileName || "").match(/Handshake_Master_Spec_v(\d+)\.(\d+)\.md/);
  if (!match) return null;
  return { major: Number(match[1]), minor: Number(match[2]) };
}

function compareVersions(a, b) {
  if (a.major !== b.major) return a.major - b.major;
  return a.minor - b.minor;
}

function findLatestSpec(repoRoot) {
  const specDir = path.join(repoRoot, GOV_ROOT_REPO_REL, "spec");
  const files = fs.readdirSync(specDir).filter((name) => /^Handshake_Master_Spec_v\d+\.\d+\.md$/.test(name));
  const parsed = files
    .map((name) => ({ name, version: parseVersion(name) }))
    .filter((entry) => entry.version)
    .sort((a, b) => compareVersions(a.version, b.version));
  if (parsed.length === 0) {
    throw new Error(`No Handshake_Master_Spec_v*.md files found in ${GOV_ROOT_REPO_REL}/spec/`);
  }
  return parsed[parsed.length - 1].name;
}

function readRequired(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Missing required file: ${filePath.replace(/\\/g, "/")}`);
  }
  return fs.readFileSync(filePath, "utf8");
}

const repoRoot = resolveRepoRoot();
process.chdir(repoRoot);

const latestSpec = findLatestSpec(repoRoot);
const errors = [];

const specCurrentPath = path.join(GOV_ROOT_REPO_REL, "spec", "SPEC_CURRENT.md");
const buildOrderPath = path.join(GOV_ROOT_REPO_REL, "roles_shared", "records", "BUILD_ORDER.md");
const pastWorkIndexPath = path.join(GOV_ROOT_REPO_REL, "reference", "PAST_WORK_INDEX.md");

const specCurrent = readRequired(specCurrentPath);
const buildOrder = readRequired(buildOrderPath);
const pastWorkIndex = readRequired(pastWorkIndexPath);

if (!specCurrent.includes(latestSpec)) {
  errors.push(`${specCurrentPath.replace(/\\/g, "/")} must point to ${latestSpec}`);
}
if (!new RegExp(`^-\\s*SPEC_TARGET:\\s*${latestSpec.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*$`, "mi").test(buildOrder)) {
  errors.push(`${buildOrderPath.replace(/\\/g, "/")} must declare SPEC_TARGET: ${latestSpec}`);
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

console.log(`spec-governance-reference-check ok: ${latestSpec}`);
