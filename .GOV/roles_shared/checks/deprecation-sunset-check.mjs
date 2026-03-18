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
    // Ignore; fall back to relative-to-file resolution.
  }
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

const repoRoot = path.resolve(resolveRepoRoot());
process.chdir(repoRoot);

const planPath = path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "DEPRECATION_SUNSET_PLAN.md");

if (!fs.existsSync(planPath)) {
  console.error(`deprecation-sunset-check: FAIL - missing ${GOV_ROOT_REPO_REL}/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`);
  process.exit(1);
}

const content = fs.readFileSync(planPath, "utf8");
const entries = content.split(/^## ENTRY\s*$/m).slice(1);
if (entries.length === 0) {
  console.error("deprecation-sunset-check: FAIL - no ## ENTRY blocks found.");
  process.exit(1);
}

const requiredFields = [
  "LEGACY_SURFACE",
  "STATUS",
  "CANONICAL_REPLACEMENT",
  "OWNER",
  "NEW_DEPENDENCIES_ALLOWED",
  "SUNSET_TRIGGER",
  "REMOVAL_ACTION",
];

const allowedStatuses = new Set(["ACTIVE_COMPAT", "REMOVED"]);
const requiredLegacySurfaces = new Set(["`WINDOWS_TERMINAL`"]);
const conditionallyRequiredLegacySurfaces = [
  {
    surface: `\`${GOV_ROOT_REPO_REL}/roles/validator/VALIDATOR_GATES.json\``,
    filePath: path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "validator", "VALIDATOR_GATES.json"),
  },
];

const seenLegacySurfaces = new Set();
const errors = [];

for (const [index, entry] of entries.entries()) {
  const label = `ENTRY[${index + 1}]`;
  for (const field of requiredFields) {
    const re = new RegExp(`^- ${field}:\\s+(.+)$`, "m");
    const match = entry.match(re);
    if (!match || !match[1].trim()) {
      errors.push(`${label}: missing ${field}`);
      continue;
    }
    if (field === "LEGACY_SURFACE") {
      seenLegacySurfaces.add(match[1].trim());
    }
    if (field === "STATUS" && !allowedStatuses.has(match[1].trim().replace(/`/g, ""))) {
      errors.push(`${label}: invalid STATUS ${match[1].trim()}`);
    }
    if (field === "NEW_DEPENDENCIES_ALLOWED") {
      const normalized = match[1].trim().replace(/`/g, "");
      if (normalized !== "YES" && normalized !== "NO") {
        errors.push(`${label}: NEW_DEPENDENCIES_ALLOWED must be YES or NO`);
      }
    }
  }
}

for (const requiredSurface of requiredLegacySurfaces) {
  if (!seenLegacySurfaces.has(requiredSurface)) {
    errors.push(`missing required legacy surface entry: ${requiredSurface}`);
  }
}

for (const entry of conditionallyRequiredLegacySurfaces) {
  if (fs.existsSync(entry.filePath) && !seenLegacySurfaces.has(entry.surface)) {
    errors.push(`missing required legacy surface entry for existing compatibility artifact: ${entry.surface}`);
  }
}

if (errors.length > 0) {
  console.error("deprecation-sunset-check: FAIL");
  console.error(errors.join("\n"));
  process.exit(1);
}

console.log("deprecation-sunset-check ok");
