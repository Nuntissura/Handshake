#!/usr/bin/env node
/**
 * kb-ready-checklist.mjs - Kernel Builder Ready-for-Validation Self-Review.
 *
 * Structured rubric the implementer MUST clear before transitioning an MT from
 * `CLAIMED` to `READY_FOR_VALIDATION`. The output is a typed
 * `KB_READY_CHECKLIST_RECEIPT` (schema:
 * `.GOV/roles_shared/schemas/KB_READY_CHECKLIST_RECEIPT.schema.json`)
 * appended to the WP communications directory.
 *
 * The rubric exists to catch latent problems that produced the prior
 * MT-046-REMEDIATOR failure: stale error messages, unconditional asserts that
 * break cross-platform CI, dead `pub` items, and missing real-resource proof.
 * See `KERNEL_BUILDER_PROTOCOL.md` section "Ready-for-Validation Self-Review"
 * and the Spec-Realism Gate.
 *
 * Two modes:
 *
 *   1. Interactive (default). Reads the MT contract, prints each rubric item
 *      with auto-derived findings, prompts for `yes`/`no`/`n/a` and an
 *      explanation, then writes the receipt.
 *
 *      just kb-ready-checklist WP-{ID} MT-{ID}
 *
 *   2. Headless JSON mode (`--json`). Prints a JSON skeleton including the
 *      auto-derived findings, and on a follow-up invocation reads a filled-in
 *      skeleton from stdin to emit the receipt. This is the only path that
 *      works inside headless ACP sessions without a TTY.
 *
 *      # 1. Print skeleton:
 *      just kb-ready-checklist WP-{ID} MT-{ID} --json
 *
 *      # 2. Fill in answers/explanations, pipe back via stdin:
 *      cat filled.json | node .GOV/roles/kernel_builder/scripts/kb-ready-checklist.mjs WP-{ID} MT-{ID} --json --emit
 *
 * Wired into fail-capture-lib per [CX-205N].
 */

import fs from "node:fs";
import path from "node:path";
import readline from "node:readline";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  normalizePath,
  repoPathAbs,
  resolveWorkPacketPath,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  COMM_ROOT,
  communicationPathsForWp,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import {
  failWithMemory,
  registerFailCaptureHook,
} from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";

const SCRIPT_NAME = "kb-ready-checklist.mjs";
const ROLE = "KERNEL_BUILDER";

registerFailCaptureHook(SCRIPT_NAME, { role: ROLE });

const SCHEMA_ID = "hsk.kb_ready_checklist_receipt@1";
const SCHEMA_VERSION = "kb_ready_checklist_receipt_v1";
const RECEIPT_KIND = "KB_READY_CHECKLIST_RECEIPT";

const RUBRIC = [
  {
    id: "RC-001-NO-STALE-REASONS",
    question:
      "Do all error messages, reason strings, and lifecycle.*_reason fields in the MT scope reflect the CURRENT state (no stale references to prior MT IDs, prior remediator session keys, or superseded approval records)?",
    auto: deriveStaleReasonFindings,
  },
  {
    id: "RC-002-NO-DEAD-CODE",
    question:
      "For every `pub` item (struct/fn/enum/trait/const) declared in the MT's owned_files, is the item referenced somewhere OUTSIDE its declaring file? Items with zero outside references are dead code unless the MT explicitly exports a public API surface.",
    auto: deriveDeadCodeFindings,
  },
  {
    id: "RC-003-CFG-GATED-TESTS",
    question:
      "For every #[test] / #[tokio::test] in the MT's owned tests/, is the gating (#[cfg(...)] / #[cfg_attr(...)]) intentional? Tests that assert platform/feature-specific invariants MUST be gated; tests intended to run in default CI MUST NOT be gated. Flag tests where the gating contradicts the assertion (e.g. asserting WINDOWS_NATIVE_JAIL_BACKEND_APPROVED without target_os=\"windows\" gate).",
    auto: deriveTestGateFindings,
  },
  {
    id: "RC-004-CROSS-PLATFORM-CI",
    question:
      "Has `cargo check` (or the project equivalent) been run for at least one non-target platform OR has cross-platform CI confirmed the build passes on platforms the MT does not natively target? Paste the command output or the CI run URL into the explanation.",
    auto: () => [],
  },
  {
    id: "RC-005-PROOF-COMMANDS",
    question:
      "Have all proof_commands declared in the MT contract been executed and returned exit-0? Paste or reference the run evidence in the explanation. (Spec-Realism Gate sub-rule 2: at least one proof command must touch the real external resource named by the contract; mocks alone do not count.)",
    auto: deriveProofCommandFindings,
  },
  {
    id: "RC-006-IMPLEMENTER-NOT-SELF-CERTIFYING",
    question:
      "Is lifecycle.claimed_by != lifecycle.completed_by (the implementer is NOT the actor that will mark the MT COMPLETED)? Per Spec-Realism Gate sub-rule 3, the implementer transitions CLAIMED -> READY_FOR_VALIDATION; the validator role transitions READY_FOR_VALIDATION -> COMPLETED.",
    auto: deriveSelfCertFindings,
  },
];

function parseArgs(argv) {
  const args = argv.slice(2);
  const positional = [];
  const flags = new Set();
  for (const arg of args) {
    if (arg.startsWith("--")) flags.add(arg);
    else positional.push(arg);
  }
  return { positional, flags };
}

function normalizeMtId(raw) {
  const cleaned = String(raw || "").trim().toUpperCase();
  if (!cleaned) return "";
  if (/^MT-[0-9A-Z._-]+$/.test(cleaned)) return cleaned;
  if (/^[0-9]+[0-9A-Z._-]*$/.test(cleaned)) return `MT-${cleaned}`;
  return cleaned;
}

function fail(message, details = [], wpId = "") {
  failWithMemory(SCRIPT_NAME, message, { role: ROLE, wpId, details });
}

function readJson(absPath) {
  return JSON.parse(fs.readFileSync(absPath, "utf8"));
}

function resolveMtContractPath(wpId, mtId) {
  const wpPaths = resolveWorkPacketPath(wpId);
  if (!wpPaths) {
    fail(`Work packet not found: ${wpId}`, [
      `Looked under ${GOV_ROOT_REPO_REL}/task_packets/ and ${GOV_ROOT_REPO_REL}/work_packets/`,
    ], wpId);
  }
  const mtAbsPath = path.join(wpPaths.packetDirAbs, `${mtId}.json`);
  if (!fs.existsSync(mtAbsPath)) {
    fail(`Microtask contract not found: ${wpId} / ${mtId}`, [
      `Looked at ${normalizePath(path.relative(REPO_ROOT, mtAbsPath))}`,
    ], wpId);
  }
  return {
    mtAbsPath,
    mtRelPath: normalizePath(path.relative(REPO_ROOT, mtAbsPath)),
    packetDirAbs: wpPaths.packetDirAbs,
    packetDirRel: wpPaths.packetDir,
  };
}

function resolveOwnedFileAbs(ownedFilePath) {
  const raw = String(ownedFilePath || "").trim();
  if (!raw) return "";
  if (path.isAbsolute(raw)) return path.resolve(raw);
  // owned_files paths are repo-relative or workspace-relative (e.g. "../handshake_main/src/...")
  return path.resolve(REPO_ROOT, raw);
}

function readOwnedFileText(ownedFilePath) {
  const absPath = resolveOwnedFileAbs(ownedFilePath);
  if (!absPath || !fs.existsSync(absPath)) return { absPath, text: "", missing: true };
  try {
    return { absPath, text: fs.readFileSync(absPath, "utf8"), missing: false };
  } catch (err) {
    return { absPath, text: "", missing: true, error: err?.message || String(err) };
  }
}

// --- Auto-finding derivations --------------------------------------------------

function deriveStaleReasonFindings(contract) {
  // Heuristic only: surface any reason/explanation strings that name a different MT ID
  // or a "REMEDIATOR-<datestamp>" / "HARDENER-<datestamp>" pattern, which is a common
  // shape for stale references after a remediation pass.
  const findings = [];
  const mtId = String(contract?.mt_id || "");
  const lifecycle = contract?.lifecycle || {};
  const reasonKeys = [
    "ready_for_validation_reason",
    "blocked_reason",
    "blocker_reason",
    "validator_verdict_reason",
    "remediation_summary",
  ];

  for (const key of reasonKeys) {
    const value = String(lifecycle[key] || "");
    if (!value) continue;
    const foreignMtRefs = Array.from(value.matchAll(/\bMT-[0-9][0-9A-Za-z._-]*\b/g))
      .map((match) => match[0])
      .filter((token) => token !== mtId);
    if (foreignMtRefs.length > 0) {
      findings.push(
        `lifecycle.${key} references foreign MT IDs: ${Array.from(new Set(foreignMtRefs)).join(", ")} — confirm each reference is still accurate.`,
      );
    }
    const stalePatterns = value.match(/(REMEDIATOR|HARDENER|FIXER)-\d{8}/g);
    if (stalePatterns) {
      findings.push(
        `lifecycle.${key} contains remediator/hardener session markers: ${stalePatterns.join(", ")} — confirm wording matches current state.`,
      );
    }
  }
  if (findings.length === 0) {
    findings.push("No obvious foreign MT or remediator references in lifecycle.*_reason fields. Re-read product-code error strings manually before answering yes.");
  }
  return findings;
}

function listPubItemsInFile(text) {
  // Conservative Rust regex pass. Looks for `pub` items at line start (allowing leading whitespace).
  const items = [];
  const patterns = [
    { kind: "pub struct", re: /^\s*pub(?:\s*\([^)]*\))?\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)/gm },
    { kind: "pub enum", re: /^\s*pub(?:\s*\([^)]*\))?\s+enum\s+([A-Za-z_][A-Za-z0-9_]*)/gm },
    { kind: "pub trait", re: /^\s*pub(?:\s*\([^)]*\))?\s+trait\s+([A-Za-z_][A-Za-z0-9_]*)/gm },
    { kind: "pub fn", re: /^\s*pub(?:\s*\([^)]*\))?\s+(?:async\s+|const\s+|unsafe\s+|extern\s+(?:"[^"]*"\s+)?)*fn\s+([A-Za-z_][A-Za-z0-9_]*)/gm },
    { kind: "pub const", re: /^\s*pub(?:\s*\([^)]*\))?\s+const\s+([A-Za-z_][A-Za-z0-9_]*)/gm },
    { kind: "pub static", re: /^\s*pub(?:\s*\([^)]*\))?\s+static\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)/gm },
    { kind: "pub type", re: /^\s*pub(?:\s*\([^)]*\))?\s+type\s+([A-Za-z_][A-Za-z0-9_]*)/gm },
  ];
  for (const { kind, re } of patterns) {
    for (const match of text.matchAll(re)) {
      items.push({ kind, name: match[1] });
    }
  }
  return items;
}

function gitGrepCount(pattern, includeGlobs, excludeAbsPath) {
  // Use `git grep -F` for fixed-string match across the active worktree.
  // Falls back to "unknown" (-1) if git is unavailable.
  try {
    const args = ["grep", "-F", "-l", "--", pattern];
    for (const glob of includeGlobs) {
      args.push(":(glob)" + glob);
    }
    const out = execFileSync("git", args, {
      cwd: REPO_ROOT,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    const files = out ? out.split(/\r?\n/).filter(Boolean) : [];
    const externalFiles = files.filter((file) => {
      const fileAbs = path.resolve(REPO_ROOT, file);
      return excludeAbsPath ? path.resolve(fileAbs) !== path.resolve(excludeAbsPath) : true;
    });
    return externalFiles.length;
  } catch {
    return -1;
  }
}

function deriveDeadCodeFindings(contract) {
  const findings = [];
  const ownedFiles = Array.isArray(contract?.owned_files) ? contract.owned_files : [];
  const rustOwned = ownedFiles.filter((entry) => /\.rs$/i.test(entry));
  if (rustOwned.length === 0) {
    findings.push("No Rust owned_files declared; dead-code grep skipped. Re-check the owned file list if this MT was expected to touch Rust.");
    return findings;
  }
  for (const owned of rustOwned) {
    const { absPath, text, missing } = readOwnedFileText(owned);
    if (missing) {
      findings.push(`${owned}: file not found at ${normalizePath(path.relative(REPO_ROOT, absPath))} — confirm path before answering.`);
      continue;
    }
    const items = listPubItemsInFile(text);
    if (items.length === 0) continue;
    for (const item of items) {
      const count = gitGrepCount(item.name, ["**/*.rs"], absPath);
      if (count === 0) {
        findings.push(`${owned}: ${item.kind} \`${item.name}\` has 0 references in other *.rs files. Confirm intentional (public API export) or remove.`);
      } else if (count === -1) {
        findings.push(`${owned}: ${item.kind} \`${item.name}\` reference count unknown (git grep failed); confirm manually.`);
      }
    }
  }
  if (findings.length === 0) {
    findings.push("Every declared `pub` item in owned Rust files has at least one reference outside its own file.");
  }
  return findings;
}

function deriveTestGateFindings(contract) {
  const findings = [];
  const ownedFiles = Array.isArray(contract?.owned_files) ? contract.owned_files : [];
  const testFiles = ownedFiles.filter((entry) =>
    /\.rs$/i.test(entry)
    && (/(^|\/)tests\//.test(entry) || /_tests?\.rs$/i.test(entry) || /(^|\/)tests?\.rs$/i.test(entry)),
  );
  if (testFiles.length === 0) {
    findings.push("No owned test files detected (paths containing `/tests/` or ending in `_tests.rs`). If this MT was expected to add tests, confirm owned_files lists them.");
    return findings;
  }
  for (const owned of testFiles) {
    const { text, missing } = readOwnedFileText(owned);
    if (missing) {
      findings.push(`${owned}: file not found; confirm path.`);
      continue;
    }
    const lines = text.split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (!/^\s*#\[(?:tokio::)?test\b/.test(line)) continue;
      // Look back for cfg attributes on the preceding non-blank lines.
      const preceding = [];
      for (let j = i - 1; j >= 0 && j >= i - 10; j--) {
        const prev = lines[j].trim();
        if (!prev) break;
        if (prev.startsWith("#[")) preceding.unshift(prev);
        else break;
      }
      const cfgAttrs = preceding.filter((attr) => /^#\[cfg(_attr)?\(/.test(attr));
      const isGated = cfgAttrs.length > 0;
      // Identify the test fn name on the next non-attribute line for clarity.
      let testName = "<unknown>";
      for (let j = i + 1; j < lines.length && j < i + 12; j++) {
        const next = lines[j].trim();
        if (next.startsWith("#[")) continue;
        const match = next.match(/fn\s+([A-Za-z_][A-Za-z0-9_]*)/);
        if (match) {
          testName = match[1];
          break;
        }
      }
      findings.push(`${owned}:${i + 1} test \`${testName}\` gating=${isGated ? cfgAttrs.join("; ") : "NONE"} — confirm intent.`);
    }
  }
  if (findings.length === 0) {
    findings.push("No #[test] / #[tokio::test] attributes detected in owned test files.");
  }
  return findings;
}

function deriveProofCommandFindings(contract) {
  const findings = [];
  const proofCommands = Array.isArray(contract?.proof_commands) ? contract.proof_commands : [];
  if (proofCommands.length === 0) {
    findings.push("MT contract declares NO proof_commands. Confirm the MT genuinely has no executable proof and the validator focus does not require runtime evidence.");
    return findings;
  }
  for (const cmd of proofCommands) {
    findings.push(`proof_command: ${cmd}`);
  }
  findings.push("This script does not execute proof_commands. Run each command before answering and reference the output (artifact path or last 5 lines) in the explanation.");
  return findings;
}

function deriveSelfCertFindings(contract) {
  const findings = [];
  const lifecycle = contract?.lifecycle || {};
  const claimedBy = String(lifecycle.claimed_by || "").trim();
  const completedBy = String(lifecycle.completed_by || "").trim();
  findings.push(`lifecycle.claimed_by = ${claimedBy || "<empty>"}`);
  findings.push(`lifecycle.completed_by = ${completedBy || "<empty>"}`);
  if (claimedBy && completedBy && claimedBy === completedBy) {
    findings.push("VIOLATION: claimed_by equals completed_by. Per Spec-Realism Gate sub-rule 3, the implementer cannot self-certify. Answer must be `no` and the MT lifecycle must be repaired before READY_FOR_VALIDATION.");
  } else if (!completedBy) {
    findings.push("completed_by is unset, which is correct at the READY_FOR_VALIDATION boundary; the validator role will populate it on transition to COMPLETED.");
  }
  return findings;
}

// --- Receipt construction ------------------------------------------------------

function buildAutoFindings(contract) {
  const map = {};
  for (const item of RUBRIC) {
    try {
      map[item.id] = item.auto(contract);
    } catch (err) {
      map[item.id] = [`auto-finding derivation failed: ${err?.message || String(err)}`];
    }
  }
  return map;
}

function buildSkeleton({ wpId, mtId, contract, mtContractPath, autoFindings }) {
  const nowIso = new Date().toISOString();
  return {
    schema_id: SCHEMA_ID,
    schema_version: SCHEMA_VERSION,
    receipt_kind: RECEIPT_KIND,
    wp_id: wpId,
    mt_id: mtId,
    actor_role: ROLE,
    actor_session: "FILL_IN_SESSION_KEY",
    generated_at_utc: nowIso,
    mt_contract_path: mtContractPath,
    summary: "FILL_IN_ONE_LINE_SUMMARY",
    overall_verdict: "PASS",
    blockers: [],
    rubric_items: RUBRIC.map((item) => ({
      rubric_item_id: item.id,
      question: item.question,
      answer: "FILL_IN_yes_no_or_n/a",
      explanation: "",
      checked_at_utc: nowIso,
      evidence_refs: [],
      auto_findings: autoFindings[item.id] || [],
    })),
  };
}

async function promptOnce(rl, prompt) {
  return new Promise((resolve) => rl.question(prompt, (answer) => resolve(answer)));
}

async function runInteractive({ wpId, mtId, contract, mtContractPath, autoFindings }) {
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout, terminal: false });
  try {
    console.log("");
    console.log(`Kernel Builder Ready-for-Validation Self-Review`);
    console.log(`WP: ${wpId}`);
    console.log(`MT: ${mtId} (${mtContractPath})`);
    console.log("");
    const sessionKey = (await promptOnce(rl, "actor_session (e.g. KERNEL_BUILDER-20260526T1830Z): ")).trim();
    if (!sessionKey) fail("actor_session is required", [], wpId);
    const summary = (await promptOnce(rl, "One-line summary of this readiness review: ")).trim();
    if (!summary) fail("summary is required", [], wpId);
    const items = [];
    let blocked = false;
    const blockers = [];
    for (const rubric of RUBRIC) {
      console.log("");
      console.log(`--- ${rubric.id} ---`);
      console.log(rubric.question);
      const findings = autoFindings[rubric.id] || [];
      if (findings.length > 0) {
        console.log("auto findings:");
        for (const finding of findings) console.log(`  - ${finding}`);
      }
      let answer = "";
      for (;;) {
        const raw = (await promptOnce(rl, "answer (yes/no/n/a): ")).trim().toLowerCase();
        if (raw === "yes" || raw === "y") { answer = "yes"; break; }
        if (raw === "no" || raw === "n") { answer = "no"; break; }
        if (raw === "n/a" || raw === "na") { answer = "n/a"; break; }
        console.log(`Invalid answer '${raw}'. Use yes/no/n/a.`);
      }
      let explanation = "";
      if (answer !== "yes") {
        explanation = (await promptOnce(rl, "explanation (required when answer is not 'yes'): ")).trim();
        if (!explanation) {
          fail(
            `Rubric ${rubric.id} answered '${answer}' without explanation; receipt blocked.`,
            [
              "Re-run the checklist with a non-empty explanation, or repair the underlying state and answer 'yes'.",
            ],
            wpId,
          );
        }
        if (answer === "no") {
          blocked = true;
          blockers.push(`${rubric.id}: ${explanation}`);
        }
      } else {
        explanation = (await promptOnce(rl, "explanation (optional evidence reference): ")).trim();
      }
      const evidenceRaw = (await promptOnce(rl, "evidence_refs (comma-separated paths or SHAs, empty for none): ")).trim();
      const evidenceRefs = evidenceRaw
        ? evidenceRaw.split(",").map((entry) => entry.trim()).filter(Boolean)
        : [];
      items.push({
        rubric_item_id: rubric.id,
        question: rubric.question,
        answer,
        explanation,
        checked_at_utc: new Date().toISOString(),
        evidence_refs: evidenceRefs,
        auto_findings: findings,
      });
    }
    const receipt = {
      schema_id: SCHEMA_ID,
      schema_version: SCHEMA_VERSION,
      receipt_kind: RECEIPT_KIND,
      wp_id: wpId,
      mt_id: mtId,
      actor_role: ROLE,
      actor_session: sessionKey,
      generated_at_utc: new Date().toISOString(),
      mt_contract_path: mtContractPath,
      summary,
      overall_verdict: blocked ? "BLOCKED" : "PASS",
      blockers,
      rubric_items: items,
    };
    return { receipt, blocked };
  } finally {
    rl.close();
  }
}

async function readStdin() {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => { data += chunk; });
    process.stdin.on("end", () => resolve(data));
    process.stdin.on("error", (err) => reject(err));
  });
}

function validateFilledSkeleton(parsed, { wpId, mtId }) {
  const errors = [];
  if (!parsed || typeof parsed !== "object") errors.push("input is not a JSON object");
  else {
    if (parsed.wp_id !== wpId) errors.push(`wp_id mismatch: skeleton=${parsed.wp_id} vs args=${wpId}`);
    if (parsed.mt_id !== mtId) errors.push(`mt_id mismatch: skeleton=${parsed.mt_id} vs args=${mtId}`);
    if (!parsed.actor_session || parsed.actor_session === "FILL_IN_SESSION_KEY") errors.push("actor_session is required");
    if (!parsed.summary || parsed.summary === "FILL_IN_ONE_LINE_SUMMARY") errors.push("summary is required");
    if (!Array.isArray(parsed.rubric_items) || parsed.rubric_items.length !== RUBRIC.length) {
      errors.push(`rubric_items must list all ${RUBRIC.length} items`);
    } else {
      for (let i = 0; i < RUBRIC.length; i++) {
        const item = parsed.rubric_items[i] || {};
        const expectedId = RUBRIC[i].id;
        if (item.rubric_item_id !== expectedId) errors.push(`rubric_items[${i}].rubric_item_id must be ${expectedId}`);
        const answer = String(item.answer || "").toLowerCase();
        if (!["yes", "no", "n/a"].includes(answer)) {
          errors.push(`rubric_items[${i}].answer must be yes/no/n/a (got ${item.answer})`);
        }
        if (answer !== "yes" && !String(item.explanation || "").trim()) {
          errors.push(`rubric_items[${i}].explanation is required when answer != yes`);
        }
      }
    }
  }
  return errors;
}

function deriveBlockersFromItems(items) {
  return items
    .filter((item) => String(item.answer || "").toLowerCase() === "no")
    .map((item) => `${item.rubric_item_id}: ${String(item.explanation || "").trim() || "<no explanation>"}`);
}

function emitReceipt({ wpId, receipt }) {
  const commPaths = communicationPathsForWp(wpId);
  const commDirAbs = repoPathAbs(commPaths.dir);
  fs.mkdirSync(commDirAbs, { recursive: true });
  const receiptsFileAbs = path.join(commDirAbs, "KB_READY_CHECKLIST_RECEIPTS.jsonl");
  const line = `${JSON.stringify(receipt)}\n`;
  fs.appendFileSync(receiptsFileAbs, line, "utf8");
  const relPath = normalizePath(path.relative(REPO_ROOT, receiptsFileAbs));
  return { absPath: receiptsFileAbs, relPath };
}

// --- Entrypoint ----------------------------------------------------------------

async function main() {
  const { positional, flags } = parseArgs(process.argv);
  if (positional.length < 2) {
    fail("Usage: kb-ready-checklist <WP_ID> <MT_ID> [--json [--emit]]", [
      "WP_ID format: WP-<name>",
      "MT_ID format: MT-<id> (or bare numeric id)",
      "--json prints the JSON skeleton (no receipt write)",
      "--json --emit reads a filled skeleton from stdin and writes the receipt",
    ]);
  }
  const wpId = String(positional[0] || "").trim();
  const mtId = normalizeMtId(positional[1]);
  if (!/^WP-/.test(wpId)) fail("Invalid WP_ID (must start with WP-)", [wpId], wpId);
  if (!/^MT-/.test(mtId)) fail("Invalid MT_ID (must start with MT-)", [mtId], wpId);

  const jsonMode = flags.has("--json");
  const emitMode = flags.has("--emit");

  const { mtAbsPath, mtRelPath } = resolveMtContractPath(wpId, mtId);
  const contract = readJson(mtAbsPath);
  const autoFindings = buildAutoFindings(contract);

  if (jsonMode && !emitMode) {
    const skeleton = buildSkeleton({ wpId, mtId, contract, mtContractPath: mtRelPath, autoFindings });
    process.stdout.write(`${JSON.stringify(skeleton, null, 2)}\n`);
    process.exit(0);
  }

  if (jsonMode && emitMode) {
    const raw = await readStdin();
    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch (err) {
      fail("--json --emit: stdin is not valid JSON", [err?.message || String(err)], wpId);
    }
    const errors = validateFilledSkeleton(parsed, { wpId, mtId });
    if (errors.length > 0) {
      fail("Filled skeleton failed validation", errors, wpId);
    }
    // Normalize answers and fill in derived fields.
    const items = parsed.rubric_items.map((item) => ({
      rubric_item_id: item.rubric_item_id,
      question: item.question || (RUBRIC.find((r) => r.id === item.rubric_item_id)?.question || ""),
      answer: String(item.answer || "").toLowerCase(),
      explanation: String(item.explanation || "").trim(),
      checked_at_utc: String(item.checked_at_utc || new Date().toISOString()),
      evidence_refs: Array.isArray(item.evidence_refs) ? item.evidence_refs : [],
      auto_findings: Array.isArray(item.auto_findings) ? item.auto_findings : (autoFindings[item.rubric_item_id] || []),
    }));
    const blockers = deriveBlockersFromItems(items);
    const receipt = {
      schema_id: SCHEMA_ID,
      schema_version: SCHEMA_VERSION,
      receipt_kind: RECEIPT_KIND,
      wp_id: wpId,
      mt_id: mtId,
      actor_role: ROLE,
      actor_session: String(parsed.actor_session || "").trim(),
      generated_at_utc: new Date().toISOString(),
      mt_contract_path: mtRelPath,
      summary: String(parsed.summary || "").trim(),
      overall_verdict: blockers.length > 0 ? "BLOCKED" : "PASS",
      blockers,
      rubric_items: items,
    };
    const { relPath } = emitReceipt({ wpId, receipt });
    process.stdout.write(`${JSON.stringify({ ok: true, verdict: receipt.overall_verdict, receipt_path: relPath, blockers }, null, 2)}\n`);
    if (receipt.overall_verdict === "BLOCKED") process.exit(2);
    return;
  }

  // Interactive mode
  if (!process.stdin.isTTY) {
    fail("Interactive mode requires a TTY. Use --json [--emit] for headless workflows.", [
      "Skeleton:  just kb-ready-checklist WP-{ID} MT-{ID} --json",
      "Emit:      cat filled.json | node .GOV/roles/kernel_builder/scripts/kb-ready-checklist.mjs WP-{ID} MT-{ID} --json --emit",
    ], wpId);
  }
  const { receipt, blocked } = await runInteractive({ wpId, mtId, contract, mtContractPath: mtRelPath, autoFindings });
  const { relPath } = emitReceipt({ wpId, receipt });
  console.log("");
  console.log(`Receipt written: ${relPath}`);
  console.log(`Verdict: ${receipt.overall_verdict}`);
  if (blocked) {
    console.log("Blockers:");
    for (const blocker of receipt.blockers) console.log(`  - ${blocker}`);
    process.exit(2);
  }
}

main().catch((err) => {
  fail(err?.message || String(err), [err?.stack || ""]);
});
