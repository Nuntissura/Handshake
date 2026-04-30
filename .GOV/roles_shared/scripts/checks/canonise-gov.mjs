#!/usr/bin/env node
/**
 * Governance Canonisation Review
 *
 * Surfaces the mandatory governance review pass for active canonisation files.
 * When governance rules or workflows change, run this to inspect the
 * file set, catch drift across protocols/shared docs/codex/operator refs,
 * and then update every applicable surface before closeout.
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
  "roles/wp_validator/WP_VALIDATOR_PROTOCOL.md",
  "roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md",
  "roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md",
  "roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md",
  "roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md",
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
  wp_validator: "roles/wp_validator/WP_VALIDATOR_PROTOCOL.md",
  integration_validator: "roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md",
  activation_manager: "roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md",
  classic_orchestrator: "roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md",
  memory_manager: "roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md",
};

// Reference checks shared by all protocols. Each role protocol SHOULD reference
// these invariants somewhere in its body so the rule is visible at session start.
const PROTOCOL_EXPECTED_REFS = [
  { pattern: "CX-130", label: "inter-role wire discipline [CX-130]" },
];

// Reference checks that apply only to the legacy three-role protocols
// (orchestrator/coder/validator) which carry the full repo-boundary boilerplate.
// The newer split protocols (wp_validator, integration_validator, etc.) inherit
// these invariants from the legacy authority files referenced in their startup
// prompts, so the checks below are scoped to the legacy three.
const LEGACY_PROTOCOL_EXPECTED_REFS = [
  { pattern: "TOOLING_GUARDRAILS", label: "TOOLING_GUARDRAILS ref" },
  { pattern: "COMMAND_SURFACE_REFERENCE", label: "COMMAND_SURFACE_REFERENCE ref" },
  { pattern: /permanent.branch/i, label: "permanent branch model" },
  { pattern: /drive.agnostic/i, label: "drive-agnostic paths" },
  { pattern: /governance.kernel/i, label: "governance kernel concept" },
  { pattern: "CX-109A", label: "no-spaces-in-names rule [CX-109A]" },
];

const LEGACY_PROTOCOLS = new Set(["orchestrator", "coder", "validator"]);

for (const [role, filePath] of Object.entries(PROTOCOLS)) {
  const content = readFile(filePath) || "";
  // Universal refs every protocol MUST carry.
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
  // Legacy-three-only refs (full boundary boilerplate).
  if (LEGACY_PROTOCOLS.has(role)) {
    for (const ref of LEGACY_PROTOCOL_EXPECTED_REFS) {
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
    const m = line.match(/^([a-z][\w-]*)\b.*:\s*$/);
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
  { pattern: "CX-130", label: "inter-role wire discipline [CX-130]" },
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
console.log("CANONISE_GOV REVIEW");
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
console.log("REQUIRED_ACTION: inspect every file in the review brief below and update applicable drift; a green summary alone is not sufficient.");

// ─── Structured review brief ─────────────────────────────────────
// Each file has a declared purpose and a scoped review directive.
// The orchestrator reads each file and checks ONLY what the directive says.
const REVIEW_BRIEF = [
  {
    file: "codex/Handshake_Codex_v1.4.md",
    purpose: "Foundational law — CX invariants, precedence rules, HARD constraints",
    directive: "Check if the governance change introduces or modifies a HARD rule that needs a new or updated CX invariant. If no new invariant is needed, skip.",
  },
  {
    file: "roles/orchestrator/ORCHESTRATOR_PROTOCOL.md",
    purpose: "Orchestrator execution rules — workflow authority, delegation, safety",
    directive: "Check if the governance change affects orchestrator workflow, delegation rules, or safety constraints. Update execution rules or 'See also' references if applicable.",
  },
  {
    file: "roles/coder/CODER_PROTOCOL.md",
    purpose: "Coder execution rules — implementation constraints, handoff evidence, scope boundaries",
    directive: "Check if the governance change introduces constraints the coder must follow during implementation or handoff. Update rules or references if applicable.",
  },
  {
    file: "roles/validator/VALIDATOR_PROTOCOL.md",
    purpose: "Classic Validator execution rules — review authority, proof standards, closeout gates (manual-relay lane)",
    directive: "Check if the governance change affects classic validation criteria, proof requirements, or closeout rules. Update rules or references if applicable.",
  },
  {
    file: "roles/wp_validator/WP_VALIDATOR_PROTOCOL.md",
    purpose: "WP Validator execution rules — per-MT boundary enforcement, scope containment, code review (orchestrator-managed lane)",
    directive: "Check if the governance change affects per-MT review, the bounded fix loop, or the WP Validator's scope. Update rules or references if applicable.",
  },
  {
    file: "roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md",
    purpose: "Integration Validator execution rules — whole-WP judgment, verdict authority, merge-to-main, sync-gov-to-main",
    directive: "Check if the governance change affects whole-WP judgment, verdict writing, merge authority, or closeout sync. Update rules or references if applicable.",
  },
  {
    file: "roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md",
    purpose: "Activation Manager execution rules — refinement, packet creation, pre-launch authoring (orchestrator-managed lane)",
    directive: "Check if the governance change affects refinement, enrichment, packet hydration, or pre-launch handback. Update rules or references if applicable.",
  },
  {
    file: "roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md",
    purpose: "Classic Orchestrator execution rules — combined pre-launch + manual relay coordination (manual-relay lane)",
    directive: "Check if the governance change affects MANUAL_RELAY workflow authority, manual relay envelopes, or the combined pre-launch flow. Update rules or references if applicable.",
  },
  {
    file: "roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md",
    purpose: "Memory Manager execution rules — memory hygiene, packetless receipt emission, proposal/flag/RGF candidate authoring",
    directive: "Check if the governance change affects memory hygiene, the memory packetless receipt schemas, or memory-manager session lifecycle. Update rules or references if applicable.",
  },
  {
    file: "roles_shared/docs/COMMAND_SURFACE_REFERENCE.md",
    purpose: "Canonical just-command documentation — grouped by workflow purpose",
    directive: "Check if the governance change adds, removes, or changes justfile recipes. Add/update/remove command entries with correct read-only/write classification.",
  },
  {
    file: "roles_shared/docs/ARCHITECTURE.md",
    purpose: "Module responsibility map — system layers, key paths, data model",
    directive: "Check if the governance change introduces a new system, module, or architectural layer. Add a row if applicable. Do not update for workflow-only changes.",
  },
  {
    file: "roles_shared/docs/START_HERE.md",
    purpose: "Navigation entrypoint — links to all key governance docs",
    directive: "Check if the governance change adds a new doc, guide, or authority source that roles need to find. Add a navigation link if applicable. Do not duplicate content from other files.",
  },
  {
    file: "roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md",
    purpose: "Compact rules index — role responsibilities, key commands, operator UX ordering",
    directive: "Check if the governance change adds commands the operator needs quick access to, or changes role responsibilities or UX ordering. Add a quickref entry if applicable.",
  },
  {
    file: "roles_shared/docs/RUNBOOK_DEBUG.md",
    purpose: "Diagnostic guide — symptoms, log locations, recovery procedures",
    directive: "Check if the governance change introduces new failure modes, log paths, or recovery steps. Add a symptom entry if applicable. Skip for non-diagnostic changes.",
  },
  {
    file: "roles_shared/docs/TOOLING_GUARDRAILS.md",
    purpose: "Append-only shared memory — recurring repo bad habits and tooling rules",
    directive: "Check if the governance change exposed a new bad habit or tooling pitfall worth recording. Append only; never edit existing entries.",
  },
  {
    file: "README.md",
    purpose: "Folder map — .GOV ownership model and directory structure",
    directive: "Check if the governance change adds or renames folders under .GOV/. Update the folder map if applicable. Skip for content-only changes.",
  },
  {
    file: "roles/README.md",
    purpose: "Role listing — directs to role-specific bundles",
    directive: "Check if a new role was added. Update listing if applicable. Skip otherwise.",
  },
  {
    file: "roles_shared/README.md",
    purpose: "Shared bucket map — shared records, runtime placement, folder inventory",
    directive: "Check if the governance change adds shared records, runtime files, or new shared folders. Update the bucket map if applicable.",
  },
];

console.log("\nSTRUCTURED REVIEW BRIEF");
console.log("(orchestrator: read each file, apply directive only when applicable)\n");
for (const entry of REVIEW_BRIEF) {
  const abs = path.resolve(GOV_ROOT_ABS, entry.file);
  const exists = fs.existsSync(abs);
  const marker = exists ? "•" : "✗ MISSING";
  console.log(`${marker} ${entry.file}`);
  console.log(`  PURPOSE:   ${entry.purpose}`);
  console.log(`  DIRECTIVE: ${entry.directive}`);
  console.log();
}

if (failCount > 0) {
  console.log("\ncanonise-gov FAIL");
  process.exit(1);
} else if (warnCount > 0) {
  console.log("\ncanonise-gov WARN (no hard failures, but docs may drift)");
} else {
  console.log("\ncanonise-gov ok");
}
