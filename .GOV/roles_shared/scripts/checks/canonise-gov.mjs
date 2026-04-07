#!/usr/bin/env node
/**
 * Governance Canonisation Audit
 *
 * Synchronises intent, rules, and instructions across governance files.
 * When governance rules or workflows change, run this to verify consistency
 * across protocols, shared docs, codex, and operator references, then
 * manually review the output file list to propagate the change.
 *
 * Checks:
 *   1. All required governance files exist
 *   2. Protocol files cross-reference shared docs and key invariants
 *   3. Navigation hub (START_HERE) links to all key docs
 *   4. Command surface + quickref document justfile recipes
 *   5. Codex references key governance mechanisms
 *   6. Operator cheat sheet references key commands
 *   7. Architecture doc covers expected layers
 *
 * Usage: just canonise-gov
 */

import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_ABS } from "../lib/runtime-paths.mjs";

const PASS = "PASS";
const WARN = "WARN";
const FAIL = "FAIL";

const results = [];
let failCount = 0;
let warnCount = 0;

function check(name, status, detail = "") {
  results.push({ name, status, detail });
  if (status === FAIL) failCount++;
  if (status === WARN) warnCount++;
}

function readFile(relPath) {
  const abs = path.resolve(GOV_ROOT_ABS, relPath);
  if (!fs.existsSync(abs)) return null;
  return fs.readFileSync(abs, "utf8");
}

// ─── 1. Required file existence ──────────────────────────────────
const REQUIRED_FILES = [
  "codex/Handshake_Codex_v1.4.md",
  "roles/orchestrator/ORCHESTRATOR_PROTOCOL.md",
  "roles/coder/CODER_PROTOCOL.md",
  "roles/validator/VALIDATOR_PROTOCOL.md",
  "roles_shared/docs/COMMAND_SURFACE_REFERENCE.md",
  "roles_shared/docs/ARCHITECTURE.md",
  "roles_shared/docs/START_HERE.md",
  "roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md",
  "roles_shared/docs/RUNBOOK_DEBUG.md",
  "roles_shared/docs/TOOLING_GUARDRAILS.md",
];

for (const f of REQUIRED_FILES) {
  const abs = path.resolve(GOV_ROOT_ABS, f);
  check(`file-exists: ${f}`, fs.existsSync(abs) ? PASS : FAIL, fs.existsSync(abs) ? "" : "missing");
}

// ─── 2. Protocol ↔ shared-doc cross-references ──────────────────
//   Every role protocol should reference key shared docs and invariants.
const PROTOCOLS = {
  orchestrator: "roles/orchestrator/ORCHESTRATOR_PROTOCOL.md",
  coder: "roles/coder/CODER_PROTOCOL.md",
  validator: "roles/validator/VALIDATOR_PROTOCOL.md",
};

const PROTOCOL_EXPECTED_REFS = [
  { pattern: "TOOLING_GUARDRAILS", label: "TOOLING_GUARDRAILS ref" },
  { pattern: "COMMAND_SURFACE_REFERENCE", label: "COMMAND_SURFACE_REFERENCE ref" },
  { pattern: /permanent.branch/i, label: "permanent branch model" },
  { pattern: /drive.agnostic/i, label: "drive-agnostic paths" },
  { pattern: /governance.kernel/i, label: "governance kernel concept" },
];

for (const [role, filePath] of Object.entries(PROTOCOLS)) {
  const content = readFile(filePath) || "";
  for (const ref of PROTOCOL_EXPECTED_REFS) {
    const found = typeof ref.pattern === "string"
      ? content.includes(ref.pattern)
      : ref.pattern.test(content);
    check(
      `${role}-protocol: ${ref.label}`,
      found ? PASS : WARN,
      found ? "" : `not found in ${role} protocol`,
    );
  }
}

// ─── 3. Navigation hub (START_HERE) completeness ─────────────────
//   START_HERE is the entrypoint — it should link to all key governance docs.
const startContent = readFile("roles_shared/docs/START_HERE.md") || "";
const START_HERE_EXPECTED = [
  { pattern: "ORCHESTRATOR_PROTOCOL", label: "orchestrator protocol link" },
  { pattern: "CODER_PROTOCOL", label: "coder protocol link" },
  { pattern: "VALIDATOR_PROTOCOL", label: "validator protocol link" },
  { pattern: /[Cc]odex/, label: "codex reference" },
  { pattern: "ARCHITECTURE", label: "architecture link" },
  { pattern: "COMMAND_SURFACE_REFERENCE", label: "command surface link" },
  { pattern: "TOOLING_GUARDRAILS", label: "tooling guardrails link" },
  { pattern: "ROLE_WORKFLOW_QUICKREF", label: "quickref link" },
  { pattern: "RUNBOOK_DEBUG", label: "runbook debug link" },
];

for (const ref of START_HERE_EXPECTED) {
  const found = typeof ref.pattern === "string"
    ? startContent.includes(ref.pattern)
    : ref.pattern.test(startContent);
  check(
    `start-here: ${ref.label}`,
    found ? PASS : WARN,
    found ? "" : "not found in START_HERE.md",
  );
}

// ─── 4. Command surface + quickref recipe coverage ───────────────
//   Every user-facing justfile recipe should be documented somewhere.
const surfaceRef = readFile("roles_shared/docs/COMMAND_SURFACE_REFERENCE.md") || "";
const quickrefContent = readFile("roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md") || "";

const justfileAbs = path.resolve(GOV_ROOT_ABS, "..", "justfile");
if (fs.existsSync(justfileAbs)) {
  const justfileContent = fs.readFileSync(justfileAbs, "utf8");
  const seen = new Set();
  for (const line of justfileContent.split("\n")) {
    const m = line.match(/^([a-z][\w-]*)\s*[^:=]*:/);
    if (!m) continue;
    const recipe = m[1];
    if (seen.has(recipe)) continue;
    seen.add(recipe);
    const documented = surfaceRef.includes(recipe) || quickrefContent.includes(recipe);
    check(
      `recipe-documented: ${recipe}`,
      documented ? PASS : WARN,
      documented ? "" : "not in command surface or quickref",
    );
  }
}

// ─── 5. Quickref governance awareness ────────────────────────────
check(
  "quickref: canonise-gov ref",
  quickrefContent.includes("canonise-gov") ? PASS : WARN,
  quickrefContent.includes("canonise-gov") ? "" : "canonise-gov not in quickref",
);
check(
  "quickref: gov-check ref",
  quickrefContent.includes("gov-check") ? PASS : WARN,
  quickrefContent.includes("gov-check") ? "" : "gov-check not in quickref",
);

// ─── 6. Codex ↔ governance alignment ─────────────────────────────
const codexContent = readFile("codex/Handshake_Codex_v1.4.md") || "";
const CODEX_EXPECTED = [
  { pattern: "canonise-gov", label: "canonise-gov reference" },
  { pattern: /governance.kernel/i, label: "governance kernel concept" },
  { pattern: /permanent.branch/i, label: "permanent branch model" },
  { pattern: /drive.agnostic/i, label: "drive-agnostic rule" },
];

for (const ref of CODEX_EXPECTED) {
  const found = typeof ref.pattern === "string"
    ? codexContent.includes(ref.pattern)
    : ref.pattern.test(codexContent);
  check(
    `codex: ${ref.label}`,
    found ? PASS : WARN,
    found ? "" : "not found in codex",
  );
}

// ─── 7. Operator cheat sheet ─────────────────────────────────────
const cheatSheetAbs = path.resolve(GOV_ROOT_ABS, "..", "..", "..", "Prompts", "Handshake_Role_Startup_Prompts.md");
const cheatSheet = fs.existsSync(cheatSheetAbs) ? fs.readFileSync(cheatSheetAbs, "utf8") : null;
if (cheatSheet) {
  const CHEAT_EXPECTED = [
    { pattern: "canonise-gov", label: "canonise-gov ref" },
    { pattern: "gov-check", label: "gov-check ref" },
    { pattern: "orchestrator-startup", label: "orchestrator-startup ref" },
  ];
  for (const ref of CHEAT_EXPECTED) {
    check(
      `cheat-sheet: ${ref.label}`,
      cheatSheet.includes(ref.pattern) ? PASS : WARN,
      cheatSheet.includes(ref.pattern) ? "" : "not in operator cheat sheet",
    );
  }
} else {
  check("cheat-sheet: file exists", WARN, `not found at ${cheatSheetAbs}`);
}

// ─── 8. Architecture doc layer coverage ──────────────────────────
const archContent = readFile("roles_shared/docs/ARCHITECTURE.md") || "";
const ARCH_EXPECTED = [
  { pattern: /[Ff]rontend|[Tt]auri|[Rr]eact/, label: "frontend layer" },
  { pattern: /[Bb]ackend|[Rr]ust/, label: "backend layer" },
  { pattern: /[Gg]overnance/, label: "governance layer" },
  { pattern: /[Ss]ession/, label: "session system" },
];

for (const ref of ARCH_EXPECTED) {
  const found = ref.pattern.test(archContent);
  check(
    `architecture: ${ref.label}`,
    found ? PASS : WARN,
    found ? "" : "not found in ARCHITECTURE.md",
  );
}

// ─── Output ──────────────────────────────────────────────────────
console.log("CANONISE_GOV AUDIT");
console.log("─".repeat(60));

const grouped = { FAIL: [], WARN: [], PASS: [] };
for (const r of results) {
  grouped[r.status].push(r);
}

if (grouped.FAIL.length > 0) {
  console.log(`\nFAILURES (${grouped.FAIL.length}):`);
  for (const r of grouped.FAIL) {
    console.log(`  ✗ ${r.name}${r.detail ? ` — ${r.detail}` : ""}`);
  }
}
if (grouped.WARN.length > 0) {
  console.log(`\nWARNINGS (${grouped.WARN.length}):`);
  for (const r of grouped.WARN) {
    console.log(`  ! ${r.name}${r.detail ? ` — ${r.detail}` : ""}`);
  }
}

console.log(`\nSUMMARY: ${results.length} checks — ${grouped.PASS.length} pass, ${warnCount} warn, ${failCount} fail`);

// ─── Review list for orchestrator ────────────────────────────────
console.log("\nREVIEW LIST (orchestrator: check these files for intent alignment):");
const reviewFiles = [
  ...REQUIRED_FILES,
  "README.md",
  "roles/README.md",
  "roles_shared/README.md",
];
for (const f of reviewFiles) {
  const abs = path.resolve(GOV_ROOT_ABS, f);
  const exists = fs.existsSync(abs);
  console.log(`  ${exists ? "•" : "✗"} ${f}`);
}

if (failCount > 0) {
  console.log("\ncanonise-gov FAIL");
  process.exit(1);
} else if (warnCount > 0) {
  console.log("\ncanonise-gov WARN (no hard failures, but docs may drift)");
} else {
  console.log("\ncanonise-gov ok");
}
