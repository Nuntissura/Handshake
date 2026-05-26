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

const PRODUCT_WORKTREE_ROOT_ENV_VAR = "HANDSHAKE_PRODUCT_WORKTREE_ROOT";

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
      "At the READY_FOR_VALIDATION boundary, is lifecycle.claimed_by set AND lifecycle.completed_by unset/empty/null? Per Spec-Realism Gate sub-rule 3, the implementer transitions CLAIMED -> READY_FOR_VALIDATION; only the validator role transitions READY_FOR_VALIDATION -> COMPLETED. A non-empty completed_by at this boundary means the implementer is fast-forwarding through validator review.",
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

// --- Cross-worktree product root resolution ---------------------------------
//
// MT contracts for kernel-build WPs declare owned_files as paths relative to
// the WP-declared product worktree (e.g. `wtc-kernel-004-fold-v1`). When this
// script runs from the gov_kernel worktree (`wt-gov-kernel`), resolving those
// paths against the local REPO_ROOT yields "file not found" for every Rust
// owned file. The auto-finders for RC-002/RC-003/RC-005 then degrade to noise.
//
// Resolution order:
//   1. `HANDSHAKE_PRODUCT_WORKTREE_ROOT` env var — operator/kbstart-set
//      explicit override. Returned with source="env".
//   2. `git worktree list --porcelain` discovery — pick the worktree whose
//      basename matches a lenient WP-ID-stem pattern derived from the WP-ID.
//      For WP-KERNEL-004-...-v1 the stem is `kernel-004` and we look for any
//      basename starting with `wtc-kernel-004`. We are deliberately lenient
//      because operator naming varies; if multiple candidates match we pick
//      the most-recently-modified by directory mtime.
//   3. Fallback to REPO_ROOT. Source="fallback-repo-root". The per-item
//      auto-finding lines downstream surface the fallback explicitly so the
//      operator knows how to fix it.
//
// The full resolution {root, source, note} is also attached to the receipt
// under `product_worktree_root_resolution` for audit by the validator role.

function deriveWpIdStem(wpId) {
  // "WP-KERNEL-004-Local-Model-Boxing-...-v1" -> "kernel-004"
  // "WP-INF-9-something" -> "inf-9"
  // Conservative: lowercase, strip leading "WP-", take the first two tokens
  // that look like a project tag + numeric/series id. Falls back to the first
  // 1-3 hyphen-delimited tokens.
  const cleaned = String(wpId || "").trim();
  if (!cleaned) return "";
  const lowered = cleaned.toLowerCase();
  const withoutPrefix = lowered.startsWith("wp-") ? lowered.slice(3) : lowered;
  const tokens = withoutPrefix.split("-").filter(Boolean);
  if (tokens.length === 0) return "";
  // First token is the project/area tag. Second token is the numeric series ID
  // when present. Anything beyond is the descriptive title and is dropped.
  if (tokens.length === 1) return tokens[0];
  const second = tokens[1];
  if (/^[0-9]+$/.test(second) || /^[0-9]+[a-z]?$/.test(second)) {
    return `${tokens[0]}-${second}`;
  }
  return tokens[0];
}

function runGitWorktreeList(repoRoot) {
  try {
    const out = execFileSync("git", ["worktree", "list", "--porcelain"], {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    return String(out || "");
  } catch {
    return "";
  }
}

function parseGitWorktreeListPorcelain(raw) {
  const entries = [];
  const lines = String(raw || "").split(/\r?\n/);
  let current = null;
  const flush = () => {
    if (current?.worktree) entries.push(current);
    current = null;
  };
  for (const line of lines) {
    if (!line.trim()) {
      flush();
      continue;
    }
    if (line.startsWith("worktree ")) {
      flush();
      current = { worktree: line.slice("worktree ".length).trim(), branch: "" };
      continue;
    }
    if (!current) continue;
    if (line.startsWith("branch ")) {
      current.branch = line.slice("branch ".length).trim();
    }
  }
  flush();
  return entries;
}

function safeStatMtimeMs(absPath) {
  try {
    return fs.statSync(absPath).mtimeMs || 0;
  } catch {
    return 0;
  }
}

export function resolveProductWorktreeRoot(repoRoot, wpId) {
  const repoRootAbs = path.resolve(String(repoRoot || ""));

  // 1. Env var override.
  const envValue = String(process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR] || "").trim();
  if (envValue) {
    const candidate = path.resolve(envValue);
    if (fs.existsSync(candidate)) {
      return {
        root: candidate,
        source: "env",
        env_var: PRODUCT_WORKTREE_ROOT_ENV_VAR,
        note: `Using product worktree root from ${PRODUCT_WORKTREE_ROOT_ENV_VAR}=${normalizePath(candidate)}.`,
      };
    }
    return {
      root: repoRootAbs,
      source: "fallback-repo-root",
      env_var: PRODUCT_WORKTREE_ROOT_ENV_VAR,
      note: `${PRODUCT_WORKTREE_ROOT_ENV_VAR} is set to '${envValue}' but that path does not exist; falling back to repo root ${normalizePath(repoRootAbs)}.`,
    };
  }

  // 2. Auto-discover via git worktree list --porcelain.
  const stem = deriveWpIdStem(wpId);
  if (stem) {
    const entries = parseGitWorktreeListPorcelain(runGitWorktreeList(repoRootAbs));
    const candidates = [];
    for (const entry of entries) {
      const wtAbs = path.resolve(entry.worktree);
      if (path.resolve(wtAbs) === repoRootAbs) continue; // skip self
      const base = path.basename(wtAbs).toLowerCase();
      const matchesPrimary = base.startsWith(`wtc-${stem}`);
      const matchesContainsWp = base.startsWith("wtc-") && base.includes(stem);
      if (matchesPrimary || matchesContainsWp) {
        candidates.push({
          root: wtAbs,
          base,
          mtimeMs: safeStatMtimeMs(wtAbs),
          branch: entry.branch || "",
        });
      }
    }
    if (candidates.length > 0) {
      // Prefer most-recently-modified to be lenient about operator naming.
      candidates.sort((a, b) => b.mtimeMs - a.mtimeMs);
      const chosen = candidates[0];
      const others = candidates.slice(1).map((c) => normalizePath(c.root));
      return {
        root: chosen.root,
        source: "git-worktree-list",
        env_var: PRODUCT_WORKTREE_ROOT_ENV_VAR,
        matched_basename: chosen.base,
        matched_stem: stem,
        other_candidates: others,
        note: `Auto-discovered product worktree '${normalizePath(chosen.root)}' via git worktree list (basename matched stem '${stem}').`,
      };
    }
  }

  // 3. Fall back to REPO_ROOT and surface the gap.
  return {
    root: repoRootAbs,
    source: "fallback-repo-root",
    env_var: PRODUCT_WORKTREE_ROOT_ENV_VAR,
    matched_stem: stem || "",
    note: `No product worktree matched WP-ID stem '${stem}' under git worktree list, and ${PRODUCT_WORKTREE_ROOT_ENV_VAR} is unset. Set ${PRODUCT_WORKTREE_ROOT_ENV_VAR} or create a 'wtc-${stem || "<wp-stem>"}*' worktree to enable cross-worktree owned-file auto-findings. Falling back to repo root ${normalizePath(repoRootAbs)}.`,
  };
}

// Module-level resolution state populated by main() before any auto-finder
// runs. Tests may call resolveProductWorktreeRoot directly without touching
// this state.
let PRODUCT_WORKTREE_RESOLUTION = {
  root: REPO_ROOT,
  source: "fallback-repo-root",
  env_var: PRODUCT_WORKTREE_ROOT_ENV_VAR,
  note: "Default (not yet initialised by main()).",
};

function setProductWorktreeResolution(resolution) {
  PRODUCT_WORKTREE_RESOLUTION = resolution;
}

function resolveOwnedFileAbs(ownedFilePath) {
  const raw = String(ownedFilePath || "").trim();
  if (!raw) return "";
  if (path.isAbsolute(raw)) return path.resolve(raw);
  // owned_files paths are product-worktree-relative (e.g. "src/backend/...").
  // Resolve against the configured product worktree root rather than the
  // gov_kernel REPO_ROOT so this script works cross-worktree.
  const base = PRODUCT_WORKTREE_RESOLUTION?.root || REPO_ROOT;
  return path.resolve(base, raw);
}

function readOwnedFileText(ownedFilePath) {
  const absPath = resolveOwnedFileAbs(ownedFilePath);
  if (!absPath || !fs.existsSync(absPath)) {
    return { absPath, text: "", missing: true };
  }
  try {
    return { absPath, text: fs.readFileSync(absPath, "utf8"), missing: false };
  } catch (err) {
    return { absPath, text: "", missing: true, error: err?.message || String(err) };
  }
}

function ownedFileMissingFinding(owned, absPath) {
  const resolution = PRODUCT_WORKTREE_RESOLUTION || {};
  const rootDisplay = normalizePath(resolution.root || REPO_ROOT);
  const absDisplay = normalizePath(absPath);
  if (resolution.source === "fallback-repo-root") {
    return (
      `${owned}: file not found at ${absDisplay}; product worktree resolution is fallback-repo-root `
      + `(${rootDisplay}). Set ${PRODUCT_WORKTREE_ROOT_ENV_VAR} or create a `
      + `wtc-* worktree matching the WP-ID to enable this auto-finding.`
    );
  }
  return `${owned}: file not found at ${absDisplay} (product worktree root: ${rootDisplay}, source=${resolution.source}). Confirm the owned_files path before answering.`;
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
  // Use `git grep -F` for fixed-string match across the active product
  // worktree. Falls back to "unknown" (-1) if git is unavailable.
  const cwd = PRODUCT_WORKTREE_RESOLUTION?.root || REPO_ROOT;
  try {
    const args = ["grep", "-F", "-l", "--", pattern];
    for (const glob of includeGlobs) {
      args.push(":(glob)" + glob);
    }
    const out = execFileSync("git", args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    const files = out ? out.split(/\r?\n/).filter(Boolean) : [];
    const externalFiles = files.filter((file) => {
      const fileAbs = path.resolve(cwd, file);
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
      findings.push(ownedFileMissingFinding(owned, absPath));
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
    const { absPath, text, missing } = readOwnedFileText(owned);
    if (missing) {
      findings.push(ownedFileMissingFinding(owned, absPath));
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
  const resolution = PRODUCT_WORKTREE_RESOLUTION || {};
  const rootDisplay = normalizePath(resolution.root || REPO_ROOT);
  findings.push(`proof_commands must be executed from the product worktree (${rootDisplay}, source=${resolution.source || "unknown"}). Manifest-path arguments in the commands below are product-worktree-relative.`);
  for (const cmd of proofCommands) {
    findings.push(`proof_command: ${cmd}`);
  }
  // Surface any product-worktree-relative file paths in the commands and
  // whether they actually exist under the resolved root. This catches the
  // common case where the commands reference a manifest or test file that
  // is not present in the active product worktree.
  const referencedPaths = new Set();
  for (const cmd of proofCommands) {
    const matches = String(cmd || "").match(/(?:^|\s)((?:src|app|tests|crates|app-tauri|backend)\/[A-Za-z0-9_\-./]+)/g);
    if (!matches) continue;
    for (const m of matches) referencedPaths.add(m.trim());
  }
  for (const ref of referencedPaths) {
    const abs = resolveOwnedFileAbs(ref);
    if (abs && fs.existsSync(abs)) {
      findings.push(`referenced path '${ref}' resolves to ${normalizePath(abs)} (exists).`);
    } else {
      findings.push(ownedFileMissingFinding(ref, abs));
    }
  }
  findings.push("This script does not execute proof_commands. Run each command before answering and reference the output (artifact path or last 5 lines) in the explanation.");
  return findings;
}

function deriveSelfCertFindings(contract) {
  // RC-006 invariant at the READY_FOR_VALIDATION boundary:
  //   - lifecycle.claimed_by MUST be set (the implementer claimed the MT);
  //   - lifecycle.completed_by MUST be unset / empty / null (only the validator
  //     role writes that field on transition to COMPLETED).
  //
  // A non-empty completed_by at READY_FOR_VALIDATION is a hard violation —
  // it means the implementer is fast-forwarding past validator review. The
  // earlier "claimed_by != completed_by" check was a structural overclaim:
  // it could only detect a violation AFTER COMPLETED was written, which is
  // already too late.
  const findings = [];
  const lifecycle = contract?.lifecycle || {};
  const claimedBy = String(lifecycle.claimed_by || "").trim();
  const completedByRaw = lifecycle.completed_by;
  const completedBy = completedByRaw === null || completedByRaw === undefined
    ? ""
    : String(completedByRaw).trim();
  findings.push(`lifecycle.claimed_by = ${claimedBy || "<empty>"}`);
  findings.push(`lifecycle.completed_by = ${completedBy || "<empty/unset>"}`);
  if (!claimedBy) {
    findings.push("VIOLATION: lifecycle.claimed_by is empty. At the READY_FOR_VALIDATION boundary the implementer must be recorded as the claimant. Answer must be `no` and the MT lifecycle must be repaired before READY_FOR_VALIDATION.");
  }
  if (completedBy) {
    findings.push(`VIOLATION: lifecycle.completed_by is set to '${completedBy}' at the READY_FOR_VALIDATION boundary. Only the validator role may write completed_by on transition to COMPLETED. This indicates the implementer is fast-forwarding through validator review. Answer must be \`no\` and the MT lifecycle must be repaired before READY_FOR_VALIDATION.`);
  }
  if (claimedBy && !completedBy) {
    findings.push("INVARIANT OK: claimed_by is set and completed_by is unset/empty — correct shape at the READY_FOR_VALIDATION boundary.");
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

function buildProductWorktreeResolutionReceipt(resolution) {
  const safe = resolution || PRODUCT_WORKTREE_RESOLUTION || {};
  return {
    root: normalizePath(safe.root || REPO_ROOT),
    source: safe.source || "unknown",
    env_var: safe.env_var || PRODUCT_WORKTREE_ROOT_ENV_VAR,
    matched_basename: safe.matched_basename || "",
    matched_stem: safe.matched_stem || "",
    other_candidates: Array.isArray(safe.other_candidates) ? safe.other_candidates : [],
    note: safe.note || "",
  };
}

function buildSkeleton({ wpId, mtId, contract, mtContractPath, autoFindings, productWorktreeResolution }) {
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
    product_worktree_root_resolution: buildProductWorktreeResolutionReceipt(productWorktreeResolution),
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

async function runInteractive({ wpId, mtId, contract, mtContractPath, autoFindings, productWorktreeResolution }) {
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout, terminal: false });
  try {
    console.log("");
    console.log(`Kernel Builder Ready-for-Validation Self-Review`);
    console.log(`WP: ${wpId}`);
    console.log(`MT: ${mtId} (${mtContractPath})`);
    const resolution = productWorktreeResolution || PRODUCT_WORKTREE_RESOLUTION;
    if (resolution) {
      console.log(`Product worktree root: ${normalizePath(resolution.root)} (source=${resolution.source})`);
      if (resolution.note) console.log(`  ${resolution.note}`);
    }
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
      product_worktree_root_resolution: buildProductWorktreeResolutionReceipt(productWorktreeResolution),
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

  // Resolve the cross-worktree product root BEFORE any auto-finder runs so
  // owned-file path resolution (RC-002 / RC-003 / RC-005) targets the correct
  // worktree. See resolveProductWorktreeRoot for the resolution order.
  const productWorktreeResolution = resolveProductWorktreeRoot(REPO_ROOT, wpId);
  setProductWorktreeResolution(productWorktreeResolution);

  const autoFindings = buildAutoFindings(contract);

  if (jsonMode && !emitMode) {
    const skeleton = buildSkeleton({
      wpId,
      mtId,
      contract,
      mtContractPath: mtRelPath,
      autoFindings,
      productWorktreeResolution,
    });
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
      product_worktree_root_resolution: buildProductWorktreeResolutionReceipt(productWorktreeResolution),
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
  const { receipt, blocked } = await runInteractive({
    wpId,
    mtId,
    contract,
    mtContractPath: mtRelPath,
    autoFindings,
    productWorktreeResolution,
  });
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

// Only auto-run main() when invoked directly as a script, not when imported
// by tests (which need the exported helpers without triggering CLI parsing).
const isEntryPoint = (() => {
  try {
    const entryArgv = process.argv[1];
    if (!entryArgv) return false;
    const entryAbs = path.resolve(entryArgv);
    const selfAbs = path.resolve(fileURLToPath(import.meta.url));
    return entryAbs === selfAbs;
  } catch {
    return false;
  }
})();

if (isEntryPoint) {
  main().catch((err) => {
    fail(err?.message || String(err), [err?.stack || ""]);
  });
}
