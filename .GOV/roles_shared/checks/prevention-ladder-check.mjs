import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  GOV_ROOT_ABS,
  GOV_ROOT_REPO_REL,
  LEGACY_SHARED_GOV_RUNTIME_ROOT,
  LEGACY_SHARED_GOV_VALIDATOR_GATES_ROOT,
  normalizePath,
} from "../scripts/lib/runtime-paths.mjs";

const PREVENTION_LADDER_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "records", "GOVERNANCE_PREVENTION_LADDER.md");
const SHIM_LEDGER_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "records", "COMPATIBILITY_SHIM_LEDGER.md");
const DEPRECATION_PLAN_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "DEPRECATION_SUNSET_PLAN.md");

const REQUIRED_ESCAPE_IDS = [
  "GOV-ESCAPE-WORKFLOW-CONTRACT-FIELDS",
  "GOV-ESCAPE-NESTED-PAYLOAD-VALIDATION",
  "GOV-ESCAPE-TIMESTAMP-TYPING",
  "GOV-ESCAPE-LEGACY-AUTHORITY-POISONING",
];

const LEGACY_RUNTIME_COMPAT_SURFACE = `${normalizePath(LEGACY_SHARED_GOV_RUNTIME_ROOT)}/*`;
const LEGACY_VALIDATOR_GATE_COMPAT_SURFACE = `${normalizePath(LEGACY_SHARED_GOV_VALIDATOR_GATES_ROOT)}/*`;
const REQUIRED_SHIM_SURFACES = [
  "WINDOWS_TERMINAL",
  LEGACY_RUNTIME_COMPAT_SURFACE,
  LEGACY_VALIDATOR_GATE_COMPAT_SURFACE,
];

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

function fail(message, details = []) {
  console.error(`prevention-ladder-check: FAIL - ${message}`);
  for (const detail of details) console.error(detail);
  process.exit(1);
}

function parseBlocks(content, heading) {
  return String(content || "").split(new RegExp(`^## ${heading}\\s*$`, "m")).slice(1);
}

function parseField(block, field) {
  const match = String(block || "").match(new RegExp(`^- ${field}:\\s+(.+)$`, "m"));
  return match ? match[1].trim() : "";
}

function stripTicks(value) {
  let normalized = String(value || "").trim();
  while (
    normalized.length >= 2
    && ((normalized.startsWith("`") && normalized.endsWith("`"))
      || (normalized.startsWith('"') && normalized.endsWith('"')))
  ) {
    normalized = normalized.slice(1, -1).trim();
  }
  return normalized;
}

function normalizeRecordPath(value) {
  return normalizePath(stripTicks(value));
}

function parsePathList(value) {
  return String(value || "")
    .split(",")
    .map((item) => normalizeRecordPath(item))
    .filter(Boolean);
}

const repoRoot = path.resolve(resolveRepoRoot());
const governanceRepoRoot = path.resolve(GOV_ROOT_ABS, "..");
process.chdir(repoRoot);

function resolveGovernancePath(relPath) {
  return path.resolve(governanceRepoRoot, normalizeRecordPath(relPath));
}

const requiredFiles = [PREVENTION_LADDER_PATH, SHIM_LEDGER_PATH, DEPRECATION_PLAN_PATH];
for (const rel of requiredFiles) {
  if (!fs.existsSync(resolveGovernancePath(rel))) {
    fail(`missing required governance record ${rel}`);
  }
}

const preventionContent = fs.readFileSync(resolveGovernancePath(PREVENTION_LADDER_PATH), "utf8");
const shimContent = fs.readFileSync(resolveGovernancePath(SHIM_LEDGER_PATH), "utf8");
const deprecationContent = fs.readFileSync(resolveGovernancePath(DEPRECATION_PLAN_PATH), "utf8");

const preventionBlocks = parseBlocks(preventionContent, "ESCAPE");
const shimBlocks = parseBlocks(shimContent, "SHIM");
if (preventionBlocks.length === 0) fail("no ## ESCAPE blocks found in Governance Prevention Ladder");
if (shimBlocks.length === 0) fail("no ## SHIM blocks found in Compatibility Shim Ledger");

const preventionRequiredFields = [
  "ESCAPE_ID",
  "STATUS",
  "TITLE",
  "TRIGGER_AUDIT",
  "FAILURE_SHAPE",
  "CANONICAL_CHECKS",
  "CANONICAL_ASSETS",
  "PROMOTION_RULE",
  "NEXT_ESCALATION",
  "EXIT_CONDITION",
];
const shimRequiredFields = [
  "SHIM_ID",
  "STATUS",
  "LEGACY_SURFACE",
  "SHIM_KIND",
  "OWNER",
  "WHY_THIS_EXISTS",
  "SUPERSEDED_BY",
  "TRACKED_IN",
  "ACTIVE_GUARDRAILS",
  "SUNSET_TRIGGER",
  "DELETION_CONDITION",
];

const preventionStatuses = new Set(["SEEDED", "ENFORCED", "MONITORED", "SUNSETTED"]);
const shimStatuses = new Set(["ACTIVE_COMPAT", "SUNSETTING", "REMOVED"]);
const errors = [];

const escapeIds = new Set();
for (const [index, block] of preventionBlocks.entries()) {
  const label = `ESCAPE[${index + 1}]`;
  for (const field of preventionRequiredFields) {
    const value = parseField(block, field);
    if (!value) errors.push(`${label}: missing ${field}`);
  }
  const escapeId = stripTicks(parseField(block, "ESCAPE_ID"));
  if (escapeId) escapeIds.add(escapeId);
  const status = stripTicks(parseField(block, "STATUS"));
  if (status && !preventionStatuses.has(status)) {
    errors.push(`${label}: invalid STATUS ${status}`);
  }
  const triggerAudit = normalizeRecordPath(parseField(block, "TRIGGER_AUDIT"));
  if (triggerAudit && !fs.existsSync(resolveGovernancePath(triggerAudit))) {
    errors.push(`${label}: TRIGGER_AUDIT path does not exist (${triggerAudit})`);
  }
  for (const rel of parsePathList(parseField(block, "CANONICAL_CHECKS"))) {
    if (rel.endsWith(".mjs") && !fs.existsSync(resolveGovernancePath(rel))) {
      errors.push(`${label}: CANONICAL_CHECKS path does not exist (${rel})`);
    }
  }
}

for (const requiredEscapeId of REQUIRED_ESCAPE_IDS) {
  if (!escapeIds.has(requiredEscapeId)) {
    errors.push(`missing required prevention entry: ${requiredEscapeId}`);
  }
}

const shimSurfaces = new Set();
for (const [index, block] of shimBlocks.entries()) {
  const label = `SHIM[${index + 1}]`;
  for (const field of shimRequiredFields) {
    const value = parseField(block, field);
    if (!value) errors.push(`${label}: missing ${field}`);
  }
  const status = stripTicks(parseField(block, "STATUS"));
  if (status && !shimStatuses.has(status)) {
    errors.push(`${label}: invalid STATUS ${status}`);
  }
  const legacySurfaceRaw = stripTicks(parseField(block, "LEGACY_SURFACE"));
  const legacySurface = legacySurfaceRaw.includes("/") ? normalizePath(legacySurfaceRaw) : legacySurfaceRaw;
  if (legacySurface) shimSurfaces.add(legacySurface);
  const trackedIn = normalizeRecordPath(parseField(block, "TRACKED_IN"));
  const normalizedDeprecationPlanPath = normalizePath(path.relative(governanceRepoRoot, resolveGovernancePath(DEPRECATION_PLAN_PATH)));
  if (trackedIn && trackedIn !== normalizedDeprecationPlanPath) {
    errors.push(`${label}: TRACKED_IN must point to ${normalizedDeprecationPlanPath}`);
  }
  for (const rel of parsePathList(parseField(block, "ACTIVE_GUARDRAILS"))) {
    if (rel.endsWith(".mjs") && !fs.existsSync(resolveGovernancePath(rel))) {
      errors.push(`${label}: ACTIVE_GUARDRAILS path does not exist (${rel})`);
    }
  }
}

for (const surface of REQUIRED_SHIM_SURFACES) {
  if (!shimSurfaces.has(surface)) {
    errors.push(`missing required compatibility shim entry: ${surface}`);
  }
}

const activeDeprecationSurfaces = parseBlocks(deprecationContent, "ENTRY")
  .map((block) => ({
    surface: (() => {
      const raw = stripTicks(parseField(block, "LEGACY_SURFACE"));
      return raw.includes("/") ? normalizePath(raw) : raw;
    })(),
    status: stripTicks(parseField(block, "STATUS")),
  }))
  .filter((entry) => entry.surface && entry.status === "ACTIVE_COMPAT");

for (const entry of activeDeprecationSurfaces) {
  if (!shimSurfaces.has(entry.surface)) {
    errors.push(`ACTIVE_COMPAT deprecation surface missing shim ledger entry: ${entry.surface}`);
  }
}

if (errors.length > 0) {
  fail("governance prevention assets are incomplete", errors);
}

console.log("prevention-ladder-check ok");
