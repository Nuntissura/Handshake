#!/usr/bin/env node
/**
 * Governance Canonisation Audit
 *
 * Audits consistency across protocol files, command surface, architecture docs,
 * operator quickref, and codex after governance refactors.
 *
 * Checks:
 *   1. Key justfile recipes are documented in COMMAND_SURFACE_REFERENCE.md
 *   2. Memory commands are referenced in all role protocols
 *   3. Snapshot system is referenced in orchestrator protocol
 *   4. ARCHITECTURE.md references key modules
 *   5. START_HERE.md links to key guides
 *   6. Quickref has current command sections
 *   7. Protocol files reference GOVERNANCE_MEMORY_GUIDE.md
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

function fileContains(relPath, pattern) {
  const content = readFile(relPath);
  if (!content) return false;
  if (typeof pattern === "string") return content.includes(pattern);
  return pattern.test(content);
}

// ─── File existence ─────────────────────────────────────────────
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
];

for (const f of REQUIRED_FILES) {
  const abs = path.resolve(GOV_ROOT_ABS, f);
  check(`file-exists: ${f}`, fs.existsSync(abs) ? PASS : FAIL, fs.existsSync(abs) ? "" : "missing");
}

// ─── Command Surface Reference completeness ─────────────────────
const EXPECTED_COMMANDS_IN_SURFACE_REF = [
  "memory-search",
  "memory-capture",
  "memory-flag",
  "memory-debug-snapshot",
  "memory-patterns",
  "memory-refresh",
  "memory-stats",
  "memory-export",
  "memory-compact",
  "memory-embed",
  "memory-hybrid-search",
  "memory-intent-snapshot",
];

const surfaceRef = readFile("roles_shared/docs/COMMAND_SURFACE_REFERENCE.md") || "";
for (const cmd of EXPECTED_COMMANDS_IN_SURFACE_REF) {
  check(
    `command-surface: ${cmd}`,
    surfaceRef.includes(cmd) ? PASS : WARN,
    surfaceRef.includes(cmd) ? "" : `not found in COMMAND_SURFACE_REFERENCE.md`,
  );
}

// ─── Protocol memory references ────────────────────────────────
const PROTOCOL_FILES = {
  "orchestrator": "roles/orchestrator/ORCHESTRATOR_PROTOCOL.md",
  "coder": "roles/coder/CODER_PROTOCOL.md",
  "validator": "roles/validator/VALIDATOR_PROTOCOL.md",
};

for (const [role, filePath] of Object.entries(PROTOCOL_FILES)) {
  const content = readFile(filePath) || "";

  check(
    `${role}-protocol: memory-search ref`,
    content.includes("memory-search") ? PASS : WARN,
    content.includes("memory-search") ? "" : "memory-search not mentioned",
  );
  check(
    `${role}-protocol: memory-capture ref`,
    content.includes("memory-capture") ? PASS : WARN,
    content.includes("memory-capture") ? "" : "memory-capture not mentioned",
  );
  check(
    `${role}-protocol: GOVERNANCE_MEMORY_GUIDE ref`,
    content.includes("GOVERNANCE_MEMORY_GUIDE") ? PASS : WARN,
    content.includes("GOVERNANCE_MEMORY_GUIDE") ? "" : "GOVERNANCE_MEMORY_GUIDE not referenced",
  );
  check(
    `${role}-protocol: snapshot awareness`,
    /snapshot|SNAPSHOT/i.test(content) ? PASS : WARN,
    /snapshot|SNAPSHOT/i.test(content) ? "" : "no snapshot/SNAPSHOT reference",
  );
  check(
    `${role}-protocol: intent-snapshot ref`,
    content.includes("intent-snapshot") ? PASS : WARN,
    content.includes("intent-snapshot") ? "" : "memory-intent-snapshot not mentioned",
  );
}

// ─── Orchestrator-specific checks ──────────────────────────────
const orchContent = readFile("roles/orchestrator/ORCHESTRATOR_PROTOCOL.md") || "";
check(
  "orchestrator-protocol: memory lifecycle section",
  orchContent.includes("Governance memory lifecycle") ? PASS : FAIL,
  orchContent.includes("Governance memory lifecycle") ? "" : "missing memory lifecycle section",
);
check(
  "orchestrator-protocol: pre-task snapshot types",
  orchContent.includes("PRE_WP_DELEGATION") ? PASS : WARN,
  orchContent.includes("PRE_WP_DELEGATION") ? "" : "snapshot types not documented",
);
check(
  "orchestrator-protocol: memory-debug-snapshot ref",
  orchContent.includes("memory-debug-snapshot") ? PASS : WARN,
  orchContent.includes("memory-debug-snapshot") ? "" : "memory-debug-snapshot not mentioned",
);

// ─── Architecture.md checks ────────────────────────────────────
const archContent = readFile("roles_shared/docs/ARCHITECTURE.md") || "";
check(
  "architecture: governance memory system row",
  archContent.includes("Governance memory system") ? PASS : FAIL,
  archContent.includes("Governance memory system") ? "" : "missing governance memory row",
);
check(
  "architecture: memory-snapshot.mjs ref",
  archContent.includes("memory-snapshot.mjs") ? PASS : WARN,
  archContent.includes("memory-snapshot.mjs") ? "" : "memory-snapshot.mjs not referenced",
);
check(
  "architecture: session-control-lib injection ref",
  archContent.includes("loadSessionMemoryLines") ? PASS : WARN,
  archContent.includes("loadSessionMemoryLines") ? "" : "session injection not referenced",
);

// ─── START_HERE.md checks ──────────────────────────────────────
const startContent = readFile("roles_shared/docs/START_HERE.md") || "";
check(
  "start-here: memory system ref",
  startContent.includes("Governance memory system") ? PASS : FAIL,
  startContent.includes("Governance memory system") ? "" : "missing memory system reference",
);
check(
  "start-here: memory guide link",
  startContent.includes("GOVERNANCE_MEMORY_GUIDE") ? PASS : WARN,
  startContent.includes("GOVERNANCE_MEMORY_GUIDE") ? "" : "missing guide link",
);

// ─── Quickref checks ───────────────────────────────────────────
const quickrefContent = readFile("roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md") || "";
check(
  "quickref: memory commands section",
  quickrefContent.includes("Memory Quick Commands") ? PASS : WARN,
  quickrefContent.includes("Memory Quick Commands") ? "" : "missing memory quick commands section",
);
check(
  "quickref: memory-debug-snapshot",
  quickrefContent.includes("memory-debug-snapshot") ? PASS : WARN,
  quickrefContent.includes("memory-debug-snapshot") ? "" : "memory-debug-snapshot not in quickref",
);
check(
  "quickref: canonise-gov ref",
  quickrefContent.includes("canonise-gov") ? PASS : WARN,
  quickrefContent.includes("canonise-gov") ? "" : "canonise-gov not in quickref",
);

// ─── canonise-gov self-referential checks ──────────────────────
// The canonise-gov command itself must be documented in key authority files
const codexContent = readFile("codex/Handshake_Codex_v1.4.md") || "";
check(
  "codex: canonise-gov ref",
  codexContent.includes("canonise-gov") ? PASS : WARN,
  codexContent.includes("canonise-gov") ? "" : "canonise-gov not mentioned in codex",
);
check(
  "orchestrator-protocol: canonise-gov ref",
  orchContent.includes("canonise-gov") ? PASS : WARN,
  orchContent.includes("canonise-gov") ? "" : "canonise-gov not mentioned in orchestrator protocol",
);
const valContent = readFile("roles/validator/VALIDATOR_PROTOCOL.md") || "";
check(
  "validator-protocol: canonise-gov ref",
  valContent.includes("canonise-gov") ? PASS : WARN,
  valContent.includes("canonise-gov") ? "" : "canonise-gov not mentioned in validator protocol",
);
check(
  "command-surface: canonise-gov ref",
  surfaceRef.includes("canonise-gov") ? PASS : WARN,
  surfaceRef.includes("canonise-gov") ? "" : "canonise-gov not in command surface reference",
);

// ─── Operator cheat sheet checks ───────────────────────────────
const cheatSheetAbs = path.resolve(GOV_ROOT_ABS, "..", "..", "..", "Prompts", "Handshake_Role_Startup_Prompts.md");
const cheatSheet = fs.existsSync(cheatSheetAbs) ? fs.readFileSync(cheatSheetAbs, "utf8") : null;
if (cheatSheet) {
  check(
    "cheat-sheet: memory-debug-snapshot ref",
    cheatSheet.includes("memory-debug-snapshot") ? PASS : WARN,
    cheatSheet.includes("memory-debug-snapshot") ? "" : "memory-debug-snapshot not in operator cheat sheet",
  );
  check(
    "cheat-sheet: canonise-gov ref",
    cheatSheet.includes("canonise-gov") ? PASS : WARN,
    cheatSheet.includes("canonise-gov") ? "" : "canonise-gov not in operator cheat sheet",
  );
  check(
    "cheat-sheet: memory-search ref",
    cheatSheet.includes("memory-search") ? PASS : WARN,
    cheatSheet.includes("memory-search") ? "" : "memory-search not in operator cheat sheet",
  );
  check(
    "cheat-sheet: memory-capture ref",
    cheatSheet.includes("memory-capture") ? PASS : WARN,
    cheatSheet.includes("memory-capture") ? "" : "memory-capture not in operator cheat sheet",
  );
  check(
    "cheat-sheet: pre-task snapshot ref",
    /snapshot/i.test(cheatSheet) ? PASS : WARN,
    /snapshot/i.test(cheatSheet) ? "" : "no snapshot reference in operator cheat sheet",
  );
  check(
    "cheat-sheet: intent-snapshot ref",
    cheatSheet.includes("intent-snapshot") ? PASS : WARN,
    cheatSheet.includes("intent-snapshot") ? "" : "memory-intent-snapshot not in operator cheat sheet",
  );
} else {
  check(
    "cheat-sheet: file exists",
    WARN,
    `not found at ${cheatSheetAbs}`,
  );
}

// ─── Cross-file justfile recipe audit ──────────────────────────
// Check that every `just memory-*` recipe in justfile has a doc ref somewhere
const justfileAbs = path.resolve(GOV_ROOT_ABS, "..", "justfile");
if (fs.existsSync(justfileAbs)) {
  const justfileContent = fs.readFileSync(justfileAbs, "utf8");
  const recipeRe = /^(memory-\S+)/gm;
  let match;
  while ((match = recipeRe.exec(justfileContent)) !== null) {
    const recipe = match[1].replace(/:.*$/, "");
    const documented = surfaceRef.includes(recipe) || quickrefContent.includes(recipe);
    check(
      `justfile-recipe-documented: ${recipe}`,
      documented ? PASS : WARN,
      documented ? "" : `${recipe} not found in command surface or quickref`,
    );
  }
}

// ─── Output ────────────────────────────────────────────────────
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

if (failCount > 0) {
  console.log("\ncanonise-gov FAIL");
  process.exit(1);
} else if (warnCount > 0) {
  console.log("\ncanonise-gov WARN (no hard failures, but docs may drift)");
} else {
  console.log("\ncanonise-gov ok");
}
