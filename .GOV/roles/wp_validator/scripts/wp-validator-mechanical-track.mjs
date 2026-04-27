#!/usr/bin/env node

import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { isInvokedAsMain } from "../../../roles_shared/scripts/lib/invocation-path-lib.mjs";
import {
  normalizeRepoPath,
  matchesAnyScopeEntry,
  parsePacketScopeList,
  parsePacketSingleField,
} from "../../../roles_shared/scripts/lib/scope-surface-lib.mjs";
import {
  REPO_ROOT,
  repoPathAbs,
  resolveWorkPacketPath,
  normalizePath,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  communicationPathsForWp,
  parseJsonFile,
  parseJsonlFile,
  validateRuntimeStatus,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import {
  listDeclaredWpMicrotasks,
  resolveDeclaredWpMicrotaskByScopeRef,
  summarizeMicrotaskFileTargetBudget,
} from "../../../roles_shared/scripts/lib/wp-microtask-lib.mjs";
import { applyMechanicalTrackRouteAnchorProjection } from "../../../roles_shared/scripts/lib/wp-review-projection-lib.mjs";
import { appendWpReceipt } from "../../../roles_shared/scripts/wp/wp-receipt-append.mjs";
import { appendWpNotification } from "../../../roles_shared/scripts/wp/wp-notification-append.mjs";
import { writeJsonFile } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";

const RESULT_SCHEMA_VERSION = "mechanical_track_result@1";
const RECEIPT_KIND = "MT_VERDICT_MECHANICAL";
const SEVERITY_VALUES = new Set(["LOW", "MEDIUM", "HIGH", "CRITICAL"]);

function parseArgs(argv = []) {
  const positionals = [];
  const flags = {
    range: "",
    json: false,
    writeReceipt: true,
    actorSession: "",
    changedFiles: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = String(argv[index] || "");
    if (arg === "--json") {
      flags.json = true;
      continue;
    }
    if (arg === "--no-receipt") {
      flags.writeReceipt = false;
      continue;
    }
    if (arg === "--range") {
      flags.range = argv[++index] || "";
      continue;
    }
    if (arg.startsWith("--range=")) {
      flags.range = arg.slice("--range=".length);
      continue;
    }
    if (arg === "--actor-session") {
      flags.actorSession = argv[++index] || "";
      continue;
    }
    if (arg.startsWith("--actor-session=")) {
      flags.actorSession = arg.slice("--actor-session=".length);
      continue;
    }
    if (arg === "--changed-files-json") {
      flags.changedFiles = JSON.parse(argv[++index] || "[]");
      continue;
    }
    if (arg.startsWith("--changed-files-json=")) {
      flags.changedFiles = JSON.parse(arg.slice("--changed-files-json=".length) || "[]");
      continue;
    }
    positionals.push(arg);
  }
  return {
    wpId: positionals[0] || "",
    mtId: positionals[1] || "",
    ...flags,
  };
}

function normalizeMtId(value = "") {
  const normalized = String(value || "").trim().toUpperCase();
  return /^MT-\d{3}$/.test(normalized) ? normalized : normalized.replace(/^MT-(\d{1,2})$/, (_m, n) => `MT-${n.padStart(3, "0")}`);
}

function concern(key, severity, evidencePath) {
  const normalizedSeverity = String(severity || "").trim().toUpperCase();
  return {
    key: String(key || "MECHANICAL_CONCERN").trim().toUpperCase(),
    severity: SEVERITY_VALUES.has(normalizedSeverity) ? normalizedSeverity : "MEDIUM",
    evidence_path: String(evidencePath || "NO_EVIDENCE").trim(),
  };
}

function shellGit(worktreeAbs, args = []) {
  return execFileSync("git", args, {
    cwd: worktreeAbs,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function resolveMaybePath(value = "", fallback = REPO_ROOT) {
  const raw = String(value || "").trim();
  if (!raw) return path.resolve(fallback);
  return path.isAbsolute(raw) ? path.resolve(raw) : repoPathAbs(raw);
}

function canonicalFsPath(value = "") {
  const resolved = path.resolve(String(value || ""));
  try {
    return fs.realpathSync.native(resolved);
  } catch {
    try {
      return fs.realpathSync(resolved);
    } catch {
      return resolved;
    }
  }
}

function loadContext(wpId) {
  const resolved = resolveWorkPacketPath(wpId);
  if (!resolved?.packetAbsPath || !fs.existsSync(resolved.packetAbsPath)) {
    throw new Error(`Official packet not found for ${wpId}`);
  }
  const packetText = fs.readFileSync(resolved.packetAbsPath, "utf8");
  const fallbackPaths = communicationPathsForWp(wpId);
  const receiptsFile = parsePacketSingleField(packetText, "WP_RECEIPTS_FILE") || fallbackPaths.receiptsFile;
  const runtimeStatusFile = parsePacketSingleField(packetText, "WP_RUNTIME_STATUS_FILE") || fallbackPaths.runtimeStatusFile;
  const worktreeDir = parsePacketSingleField(packetText, "LOCAL_WORKTREE_DIR") || REPO_ROOT;
  const commDir = parsePacketSingleField(packetText, "WP_COMMUNICATION_DIR")
    || normalizePath(path.dirname(receiptsFile));
  return {
    wpId,
    packetPath: resolved.packetPath,
    packetAbsPath: resolved.packetAbsPath,
    packetDir: resolved.packetDir,
    packetText,
    receiptsFile: normalizePath(receiptsFile),
    receiptsAbsPath: repoPathAbs(receiptsFile),
    runtimeStatusFile: normalizePath(runtimeStatusFile),
    runtimeStatusAbsPath: repoPathAbs(runtimeStatusFile),
    commDir: normalizePath(commDir),
    commDirAbs: repoPathAbs(commDir),
    worktreeDir: normalizePath(worktreeDir),
    worktreeAbs: resolveMaybePath(worktreeDir, REPO_ROOT),
    branch: parsePacketSingleField(packetText, "LOCAL_BRANCH") || null,
  };
}

function readChangedFiles(worktreeAbs, range = "", injectedChangedFiles = null) {
  if (Array.isArray(injectedChangedFiles)) {
    return injectedChangedFiles.map((entry) => normalizeRepoPath(entry)).filter(Boolean);
  }
  const normalizedRange = String(range || "").trim();
  try {
    const output = normalizedRange
      ? shellGit(worktreeAbs, ["diff", "--name-only", normalizedRange])
      : shellGit(worktreeAbs, ["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]);
    return output.split(/\r?\n/).map((line) => normalizeRepoPath(line)).filter(Boolean);
  } catch {
    const output = shellGit(worktreeAbs, ["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]);
    return output.split(/\r?\n/).map((line) => normalizeRepoPath(line)).filter(Boolean);
  }
}

function worktreeConfinement(context, injectedHead = "") {
  const issues = [];
  if (!fs.existsSync(context.worktreeAbs)) {
    issues.push("worktree_missing");
    return {
      ok: false,
      status: "FAIL",
      worktree_dir: normalizePath(context.worktreeAbs),
      git_root: null,
      head: null,
      issues,
    };
  }
  let gitRoot = "";
  let head = String(injectedHead || "").trim();
  try {
    gitRoot = shellGit(context.worktreeAbs, ["rev-parse", "--show-toplevel"]);
    if (!head) head = shellGit(context.worktreeAbs, ["rev-parse", "HEAD"]);
  } catch {
    issues.push("git_metadata_unavailable");
  }
  if (gitRoot && canonicalFsPath(gitRoot) !== canonicalFsPath(context.worktreeAbs)) {
    issues.push("worktree_dir_not_git_root");
  }
  return {
    ok: issues.length === 0,
    status: issues.length === 0 ? "PASS" : "FAIL",
    worktree_dir: normalizePath(context.worktreeAbs),
    git_root: gitRoot ? normalizePath(gitRoot) : null,
    head: head || null,
    issues,
  };
}

function evaluateFileList({ mtDefinition, changedFiles }) {
  const budget = summarizeMicrotaskFileTargetBudget(changedFiles, mtDefinition);
  const issues = [];
  if (!mtDefinition) issues.push("mt_contract_missing");
  if (changedFiles.length === 0) issues.push("no_changed_files");
  if ((mtDefinition?.codeSurfaces || []).length === 0) issues.push("mt_code_surfaces_missing");
  if (budget.outOfBudgetTargets.length > 0) issues.push("changed_files_outside_mt_code_surfaces");
  return {
    ok: issues.length === 0,
    status: issues.length === 0 ? "PASS" : "FAIL",
    changed_files: changedFiles,
    declared_code_surfaces: mtDefinition?.codeSurfaces || [],
    out_of_contract_files: budget.outOfBudgetTargets,
    issues,
  };
}

function evaluateScope({ packetText, changedFiles }) {
  const inScopePaths = parsePacketScopeList(packetText, "IN_SCOPE_PATHS", { stopLabels: ["OUT_OF_SCOPE"] });
  const outOfScope = inScopePaths.length === 0
    ? []
    : changedFiles.filter((entry) => !matchesAnyScopeEntry(entry, inScopePaths));
  const status = inScopePaths.length === 0 ? "NOT_DECLARED" : outOfScope.length === 0 ? "PASS" : "FAIL";
  return {
    ok: status !== "FAIL",
    status,
    in_scope_paths: inScopePaths,
    out_of_scope_files: outOfScope,
    issues: outOfScope.length > 0 ? ["changed_files_outside_packet_in_scope_paths"] : [],
  };
}

function evaluateBoundary(fileListResult) {
  return {
    ok: fileListResult.ok,
    status: fileListResult.ok ? "PASS" : "FAIL",
    outside_files: fileListResult.out_of_contract_files,
    declared_boundary: fileListResult.declared_code_surfaces,
    issues: fileListResult.issues,
  };
}

function readCompileGateLog(context) {
  const file = path.join(context.commDirAbs, "COMPILE_GATE_LOG.jsonl");
  if (!fs.existsSync(file)) return [];
  return fs.readFileSync(file, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return null;
      }
    })
    .filter(Boolean);
}

function evaluateBuildEvidence({ context, mtId, head }) {
  const entries = readCompileGateLog(context)
    .filter((entry) => String(entry?.mt_id || "").trim().toUpperCase() === mtId)
    .filter((entry) => !head || !entry?.commit || String(entry.commit || "").trim() === head)
    .sort((left, right) => String(right.timestamp || "").localeCompare(String(left.timestamp || "")));
  const latest = entries[0] || null;
  if (!latest) {
    return {
      ok: true,
      status: "NOT_FOUND",
      gate: null,
      commit: head || null,
      evidence_path: normalizePath(path.join(context.commDir, "COMPILE_GATE_LOG.jsonl")),
      issues: ["compile_gate_evidence_not_found"],
    };
  }
  const gate = String(latest.gate || "").trim().toUpperCase();
  return {
    ok: gate !== "COMPILE_FAILED",
    status: gate === "COMPILE_FAILED" ? "FAIL" : "PASS",
    gate,
    commit: latest.commit || head || null,
    evidence_path: normalizePath(path.join(context.commDir, "COMPILE_GATE_LOG.jsonl")),
    issues: gate === "COMPILE_FAILED" ? ["compile_gate_failed"] : [],
  };
}

function concernStrings(concerns = []) {
  return concerns.map((item) => `${item.key}:${item.severity}:${item.evidence_path}`);
}

function receiptMechanicalResult(result) {
  return {
    mt_id: result.mt_id,
    verdict: result.verdict,
    concerns: result.concerns,
    boundary_check_result: result.boundary_check_result,
    scope_check_result: result.scope_check_result,
    file_list_match_result: result.file_list_match_result,
    build_pass_evidence: result.build_pass_evidence,
    helper_invocation_id: result.helper_invocation_id,
  };
}

function deriveHelperInvocationId({ wpId, mtId, range, head, changedFiles }) {
  const hash = crypto.createHash("sha256");
  hash.update(JSON.stringify({
    wpId,
    mtId,
    range: range || "",
    head: head || "",
    changedFiles: [...changedFiles].sort(),
  }));
  return `mt-mech-${hash.digest("hex").slice(0, 16)}`;
}

function latestExistingMechanicalReceipt(receipts = [], helperInvocationId = "") {
  return [...receipts]
    .reverse()
    .find((entry) =>
      String(entry?.receipt_kind || "").trim().toUpperCase() === RECEIPT_KIND
      && String(entry?.mechanical_result?.helper_invocation_id || "").trim() === helperInvocationId
    ) || null;
}

function sessionForRole(runtimeStatus = {}, role = "") {
  const normalizedRole = String(role || "").trim().toUpperCase();
  const sessions = runtimeStatus?.active_role_sessions;
  if (sessions && typeof sessions === "object" && !Array.isArray(sessions)) {
    const candidate = sessions[normalizedRole];
    if (candidate && typeof candidate === "object") {
      return String(candidate.session_id || candidate.session || candidate.id || "").trim();
    }
    if (typeof candidate === "string") return candidate.trim();
  }
  return "";
}

function appendMechanicalReceipt({ context, result, actorSession, targetSession }) {
  const targetRole = result.verdict === "FAIL" ? "CODER" : "WP_VALIDATOR";
  const summary = `${result.mt_id} mechanical ${result.verdict}: ${result.concerns.length} concern(s)`;
  return appendWpReceipt({
    wpId: context.wpId,
    actorRole: "WP_VALIDATOR",
    actorSession,
    receiptKind: RECEIPT_KIND,
    summary,
    stateBefore: "MECHANICAL_TRACK_PENDING",
    stateAfter: result.verdict === "PASS" ? "MECHANICAL_TRACK_PASSED" : "MECHANICAL_TRACK_FAILED",
    refs: [context.packetPath, result.mt_packet_path].filter(Boolean),
    branch: context.branch,
    worktreeDir: context.worktreeDir,
    targetRole,
    targetSession,
    correlationId: result.correlation_id,
    packetRowRef: result.mt_id,
    microtaskContract: {
      scope_ref: result.mt_id,
      file_targets: result.file_list_match_result.declared_code_surfaces,
      proof_commands: result.expected_tests,
      expected_receipt_kind: RECEIPT_KIND,
      review_mode: "BLOCKING",
      phase_gate: "MICROTASK",
      review_outcome: result.verdict === "FAIL" ? "REPAIR_REQUIRED" : "UNKNOWN",
    },
    mechanicalResult: receiptMechanicalResult(result),
    verb: "MT_VERDICT",
    verbBody: {
      mt_id: result.mt_id,
      verdict: result.verdict,
      concerns: result.concerns,
      track: "MECHANICAL",
    },
  }, { autoRelay: false });
}

function projectRuntimeAfterMechanical({ context, result, runtimeStatus, targetSession }) {
  if (!runtimeStatus || !context.runtimeStatusAbsPath || !fs.existsSync(context.runtimeStatusAbsPath)) {
    return null;
  }
  const nextRuntime = applyMechanicalTrackRouteAnchorProjection(runtimeStatus, result, {
    correlationId: result.correlation_id,
    targetSession,
  });
  nextRuntime.last_event = `receipt_${RECEIPT_KIND.toLowerCase()}`;
  nextRuntime.last_event_at = result.timestamp_utc;
  if (result.verdict === "FAIL") {
    nextRuntime.next_expected_actor = "CODER";
    nextRuntime.next_expected_session = targetSession || nextRuntime.next_expected_session || null;
    nextRuntime.waiting_on = "MT_MECHANICAL_REMEDIATION";
    nextRuntime.waiting_on_session = targetSession || nextRuntime.waiting_on_session || null;
    nextRuntime.validator_trigger = "NONE";
    nextRuntime.validator_trigger_reason = `${result.mt_id} mechanical track failed`;
    nextRuntime.attention_required = true;
    nextRuntime.ready_for_validation = false;
    nextRuntime.ready_for_validation_reason = null;
  } else {
    nextRuntime.next_expected_actor = "WP_VALIDATOR";
    nextRuntime.next_expected_session = targetSession || nextRuntime.next_expected_session || null;
    nextRuntime.waiting_on = "MT_JUDGMENT_REVIEW";
    nextRuntime.waiting_on_session = targetSession || nextRuntime.waiting_on_session || null;
    nextRuntime.validator_trigger = "MICROTASK_REVIEW_READY";
    nextRuntime.validator_trigger_reason = `${result.mt_id} mechanical track passed; judgment review remains required`;
  }
  const errors = validateRuntimeStatus(nextRuntime);
  if (errors.length > 0) {
    throw new Error(`Runtime status validation failed after mechanical projection: ${errors.join("; ")}`);
  }
  writeJsonFile(context.runtimeStatusAbsPath, nextRuntime);
  return nextRuntime;
}

function appendFailNotification({ context, result, actorSession, targetSession }) {
  if (result.verdict !== "FAIL") return null;
  return appendWpNotification({
    wpId: context.wpId,
    sourceKind: RECEIPT_KIND,
    sourceRole: "WP_VALIDATOR",
    sourceSession: actorSession,
    targetRole: "CODER",
    targetSession,
    correlationId: result.correlation_id,
    summary: `${result.mt_id} mechanical track failed: ${concernStrings(result.concerns).join("; ")}`,
    timestamp: result.timestamp_utc,
  }, { autoRelay: true });
}

export async function runMechanicalTrack({
  wpId = "",
  mtId = "",
  range = "",
  changedFiles = null,
  writeReceipt = true,
  actorSession = "",
  gitHead = "",
  timestamp = null,
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  const normalizedMtId = normalizeMtId(mtId);
  if (!/^WP-/.test(normalizedWpId)) throw new Error("wpId is required");
  if (!/^MT-\d{3}$/.test(normalizedMtId)) throw new Error("mtId must look like MT-001");

  const context = loadContext(normalizedWpId);
  const runtimeStatus = fs.existsSync(context.runtimeStatusAbsPath)
    ? parseJsonFile(context.runtimeStatusFile)
    : null;
  const declaredMicrotasks = listDeclaredWpMicrotasks(normalizedWpId);
  const resolution = resolveDeclaredWpMicrotaskByScopeRef(normalizedWpId, normalizedMtId, declaredMicrotasks);
  const mtDefinition = resolution.match;
  const confinement = worktreeConfinement(context, gitHead);
  const files = confinement.ok
    ? readChangedFiles(context.worktreeAbs, range, changedFiles)
    : (Array.isArray(changedFiles) ? changedFiles.map((entry) => normalizeRepoPath(entry)).filter(Boolean) : []);
  const fileList = evaluateFileList({ mtDefinition, changedFiles: files });
  const scope = evaluateScope({ packetText: context.packetText, changedFiles: files });
  const boundary = evaluateBoundary(fileList);
  const buildEvidence = evaluateBuildEvidence({
    context,
    mtId: normalizedMtId,
    head: confinement.head,
  });

  const concerns = [];
  if (!mtDefinition) concerns.push(concern("MT_CONTRACT_MISSING", "HIGH", context.packetPath));
  if (!confinement.ok) concerns.push(...confinement.issues.map((item) => concern(item, "HIGH", context.worktreeDir)));
  if (!fileList.ok) concerns.push(...fileList.issues.map((item) => concern(item, item === "no_changed_files" ? "MEDIUM" : "HIGH", mtDefinition?.packetPath || context.packetPath)));
  if (!scope.ok) concerns.push(...scope.issues.map((item) => concern(item, "HIGH", context.packetPath)));
  if (!buildEvidence.ok) concerns.push(...buildEvidence.issues.map((item) => concern(item, "HIGH", buildEvidence.evidence_path)));
  if (buildEvidence.status === "NOT_FOUND") {
    concerns.push(concern("BUILD_PASS_EVIDENCE_NOT_FOUND", "LOW", buildEvidence.evidence_path));
  }

  const verdict = [confinement.ok, fileList.ok, scope.ok, boundary.ok, buildEvidence.ok].every(Boolean)
    ? "PASS"
    : "FAIL";
  const helperInvocationId = deriveHelperInvocationId({
    wpId: normalizedWpId,
    mtId: normalizedMtId,
    range,
    head: confinement.head,
    changedFiles: files,
  });
  const correlationId = `MECH-${normalizedMtId}-${helperInvocationId.slice(-8)}`;
  const now = String(timestamp || new Date().toISOString());
  const result = {
    schema_version: RESULT_SCHEMA_VERSION,
    timestamp_utc: now,
    wp_id: normalizedWpId,
    mt_id: normalizedMtId,
    mt_packet_path: mtDefinition?.packetPath || "",
    expected_tests: mtDefinition?.expectedTests || [],
    verdict,
    concerns,
    changed_files: files,
    range: String(range || "").trim() || null,
    commit: confinement.head,
    correlation_id: correlationId,
    worktree_confinement_result: confinement,
    boundary_check_result: boundary,
    scope_check_result: scope,
    file_list_match_result: fileList,
    build_pass_evidence: buildEvidence,
    helper_invocation_id: helperInvocationId,
    receipt_written: false,
    idempotent_replay: false,
  };

  const resolvedActorSession = String(actorSession || "").trim() || `WP_VALIDATOR_MECHANICAL:${normalizedWpId}`;
  const coderSession = sessionForRole(runtimeStatus, "CODER") || `CODER:${normalizedWpId}`;
  const wpValidatorSession = sessionForRole(runtimeStatus, "WP_VALIDATOR") || `WP_VALIDATOR:${normalizedWpId}`;
  const targetSession = verdict === "FAIL" ? coderSession : wpValidatorSession;

  if (writeReceipt) {
    const receipts = fs.existsSync(context.receiptsAbsPath) ? parseJsonlFile(context.receiptsFile) : [];
    const existingReceipt = latestExistingMechanicalReceipt(receipts, helperInvocationId);
    if (existingReceipt) {
      result.receipt_written = false;
      result.idempotent_replay = true;
    } else {
      appendMechanicalReceipt({ context, result, actorSession: resolvedActorSession, targetSession });
      const currentRuntime = fs.existsSync(context.runtimeStatusAbsPath)
        ? parseJsonFile(context.runtimeStatusFile)
        : runtimeStatus;
      projectRuntimeAfterMechanical({ context, result, runtimeStatus: currentRuntime, targetSession });
      appendFailNotification({ context, result, actorSession: resolvedActorSession, targetSession });
      result.receipt_written = true;
    }
  }

  result.content = `${normalizedMtId} mechanical ${verdict}: ${concerns.length} concern(s)`;
  return result;
}

async function runCli() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.wpId || !args.mtId) {
    console.error("Usage: node .GOV/roles/wp_validator/scripts/wp-validator-mechanical-track.mjs WP-{ID} MT-NNN [--range <rev-range>] [--json] [--no-receipt]");
    process.exit(1);
  }
  const result = await runMechanicalTrack(args);
  if (args.json) {
    process.stdout.write(`${JSON.stringify({ content: result.content, details: result }, null, 2)}\n`);
  } else {
    console.log(`[WP_VALIDATOR_MECHANICAL] ${result.content}`);
    console.log(`[WP_VALIDATOR_MECHANICAL] helper_invocation_id=${result.helper_invocation_id}`);
    if (result.concerns.length > 0) {
      console.log(`[WP_VALIDATOR_MECHANICAL] concerns=${concernStrings(result.concerns).join("; ")}`);
    }
  }
  process.exit(result.verdict === "FAIL" ? 2 : 0);
}

if (isInvokedAsMain(import.meta.url, process.argv[1])) {
  runCli().catch((error) => {
    console.error(`[WP_VALIDATOR_MECHANICAL] failed: ${error?.message || String(error)}`);
    process.exit(1);
  });
}
