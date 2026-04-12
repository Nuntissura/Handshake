#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionRegistry,
  registryBatchLaunchSummary,
  registrySessionSummary,
} from "../session/session-registry-lib.mjs";
import {
  SESSION_CONTROL_BROKER_STATE_FILE,
  SESSION_CONTROL_OUTPUT_DIR,
  SESSION_CONTROL_REQUESTS_FILE,
  SESSION_CONTROL_RESULTS_FILE,
  SESSION_REGISTRY_FILE,
} from "../session/session-policy.mjs";
import {
  computedPolicyOutcomeAllowsClosure,
  evaluateComputedPolicyGateFromPacketText,
} from "../lib/computed-policy-gate-lib.mjs";
import {
  evaluateWpCommunicationBoundary,
  evaluateWpCommunicationHealth,
} from "../lib/wp-communication-health-lib.mjs";
import {
  NOTIFICATION_CURSOR_FILE_NAME,
  NOTIFICATIONS_FILE_NAME,
  REVIEW_TRACKED_RECEIPT_KIND_VALUES,
  parseJsonFile,
  parseJsonlFile,
} from "../lib/wp-communications-lib.mjs";
import { resolveValidatorGatePath } from "../lib/validator-gate-paths.mjs";
import {
  repoPathAbs,
  resolveOrchestratorGatesPath,
  resolveRefinementPath,
  resolveWorkPacketPath,
} from "../lib/runtime-paths.mjs";
import { getCurrentSession } from "../memory/governance-memory-lib.mjs";
import { checkAllNotifications } from "../wp/wp-check-notifications.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("generate-post-run-audit-skeleton.mjs", { role: "SHARED" });

function fail(message) {
  failWithMemory("generate-post-run-audit-skeleton.mjs", message, { role: "SHARED" });
}

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

const REPORT_TIMEZONE = "Europe/Brussels";

function dateTimeParts(value = new Date(), timeZone = REPORT_TIMEZONE) {
  const date = value instanceof Date ? value : new Date(value);
  return Object.fromEntries(
    new Intl.DateTimeFormat("en-CA", {
      timeZone,
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hourCycle: "h23",
    }).formatToParts(date)
      .filter((part) => part.type !== "literal")
      .map((part) => [part.type, part.value]),
  );
}

function formatLocalDate(value = new Date(), timeZone = REPORT_TIMEZONE) {
  const parts = dateTimeParts(value, timeZone);
  return `${parts.year}-${parts.month}-${parts.day}`;
}

function formatLocalTimestamp(value = new Date(), timeZone = REPORT_TIMEZONE) {
  const parts = dateTimeParts(value, timeZone);
  return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}:${parts.second} ${timeZone}`;
}

function formatDateTag(value = new Date(), timeZone = REPORT_TIMEZONE) {
  const parts = dateTimeParts(value, timeZone);
  return `${parts.year}${parts.month}${parts.day}`;
}

function auditSlugFromWpId(wpId) {
  return String(wpId || "")
    .replace(/^WP-\d+-/i, "")
    .replace(/-v\d+$/i, "")
    .replace(/[^A-Za-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .toUpperCase();
}

function auditSlugHyphen(wpId) {
  return auditSlugFromWpId(wpId).replace(/_/g, "-");
}

function countBy(items, predicate) {
  return (items || []).reduce((sum, item) => sum + (predicate(item) ? 1 : 0), 0);
}

function parseArgs(argv) {
  const args = [...argv];
  const options = {
    wpId: "",
    mode: "post-run",
    output: "",
    autoOutput: false,
    force: false,
    sessionIntention: "",
  };

  while (args.length > 0) {
    const token = String(args.shift() || "").trim();
    if (!token) continue;
    if (!options.wpId && /^WP-/.test(token)) {
      options.wpId = token;
      continue;
    }
    if (token === "--mode") {
      options.mode = String(args.shift() || "").trim().toLowerCase();
      continue;
    }
    if (token === "--output") {
      options.output = String(args.shift() || "").trim();
      continue;
    }
    if (token === "--auto-output") {
      options.autoOutput = true;
      continue;
    }
    if (token === "--force") {
      options.force = true;
      continue;
    }
    if (token === "--session-intention") {
      options.sessionIntention = String(args.shift() || "").trim();
      continue;
    }
    fail(`Unknown argument: ${token}`);
  }

  if (!options.wpId) {
    fail("Usage: node .GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs WP-{ID} [--mode post-run|live] [--output <file.md>] [--auto-output] [--force] [--session-intention \"...\"]");
  }
  if (!["post-run", "live"].includes(options.mode)) {
    fail(`Unknown mode: ${options.mode}`);
  }
  if (options.output && options.autoOutput) {
    fail("Use either --output or --auto-output, not both.");
  }

  return options;
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function parsePacketStatus(text) {
  return (
    (String(text || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(text || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || ""
  ).trim();
}

function taskBoardStatus(governanceRepoRoot, wpId) {
  const boardPath = path.join(governanceRepoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md");
  if (!fs.existsSync(boardPath)) return "";
  const content = fs.readFileSync(boardPath, "utf8");
  const match = content.match(new RegExp(`- \\*\\*\\[${wpId.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\$&")}\\]\\*\\* - \\[([^\\]]+)\\]`, "i"));
  return match ? match[1].trim() : "";
}

function readBrokerState(repoRoot) {
  const brokerPath = path.resolve(repoRoot, SESSION_CONTROL_BROKER_STATE_FILE);
  const broker = fs.existsSync(brokerPath) ? parseJsonFile(brokerPath) : null;
  const activeRuns = Array.isArray(broker?.active_runs) ? broker.active_runs : [];
  return {
    path: normalizePath(SESSION_CONTROL_BROKER_STATE_FILE),
    outputDir: normalizePath(SESSION_CONTROL_OUTPUT_DIR),
    exists: Boolean(broker),
    buildId: broker?.broker_build_id || "NONE",
    authMode: broker?.broker_auth_mode || "NONE",
    brokerPid: broker?.broker_pid || 0,
    host: broker?.host || "NONE",
    port: broker?.port || 0,
    updatedAt: broker?.updated_at || "NONE",
    activeRuns,
  };
}

function formatBrokerActiveRuns(activeRuns) {
  if (!activeRuns || activeRuns.length === 0) return ["- NONE"];
  return activeRuns.map((run) =>
    `- ${run.role || "ROLE"} ${run.command_kind || "COMMAND"} | session=${run.session_key || "NONE"} | started=${run.started_at || "NONE"} | timeout=${run.timeout_at || "NONE"}`
  );
}

function listMicrotasks(packetAbsPath) {
  const packetDir = path.dirname(packetAbsPath);
  if (!fs.existsSync(packetDir)) return [];
  return fs.readdirSync(packetDir)
    .filter((entry) => /^MT-\d+\.md$/i.test(entry))
    .sort((left, right) => left.localeCompare(right, "en", { numeric: true }))
    .map((entry) => entry.replace(/\.md$/i, ""));
}

function buildPerMicrotaskSeedRows(microtasks) {
  if (!microtasks || microtasks.length === 0) {
    return ["MICROTASKS_NOT_USED or MT files not yet generated"];
  }
  return microtasks.map((mtId) => `| ${mtId} | <pending> | NONE | NOT_SENT | N/A | N/A | NO | 0 |`);
}

function formatDisplayTimeFromIso(iso, timeZone = REPORT_TIMEZONE) {
  if (!iso) return "N/A";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "N/A";
  return formatLocalTimestamp(date, timeZone);
}

function defaultLiveReviewRelativePath(governanceRepoRoot, wpId, now = new Date()) {
  return normalizePath(path.relative(
    governanceRepoRoot,
    path.join(
      governanceRepoRoot,
      ".GOV",
      "Audits",
      "smoketest",
      `DOSSIER_${formatDateTag(now)}_${auditSlugFromWpId(wpId)}_WORKFLOW_DOSSIER.md`,
    ),
  ));
}

function findExistingLiveReviewRelativePath(governanceRepoRoot, wpId) {
  const auditsDir = path.join(governanceRepoRoot, ".GOV", "Audits", "smoketest");
  if (!fs.existsSync(auditsDir)) return "";
  const matches = fs.readdirSync(auditsDir)
    .filter((entry) => entry.toLowerCase().endsWith(".md"))
    .map((entry) => {
      const absPath = path.join(auditsDir, entry);
      try {
        const text = fs.readFileSync(absPath, "utf8");
        const isTargetPacket = text.includes(`- ACTIVE_RECOVERY_PACKET: ${wpId}`);
        const isLive = text.includes("- LIVE_REVIEW_STATUS: OPEN");
        return isTargetPacket && isLive
          ? { absPath, mtimeMs: fs.statSync(absPath).mtimeMs }
          : null;
      } catch {
        return null;
      }
    })
    .filter(Boolean)
    .sort((left, right) => right.mtimeMs - left.mtimeMs);
  if (matches.length === 0) return "";
  return normalizePath(path.relative(governanceRepoRoot, matches[0].absPath));
}

function readThreadSummary(threadPath) {
  const threadAbsPath = threadPath ? repoPathAbs(threadPath) : "";
  if (!threadAbsPath || !fs.existsSync(threadAbsPath)) {
    return { exists: false, lineCount: 0, lastNonEmptyLine: "" };
  }
  const lines = fs.readFileSync(threadAbsPath, "utf8").split(/\r?\n/);
  const nonEmpty = lines.map((line) => line.trim()).filter(Boolean);
  return {
    exists: true,
    lineCount: lines.length,
    lastNonEmptyLine: nonEmpty.at(-1) || "",
  };
}

function formatReceiptSummary(receipts) {
  const total = receipts.length;
  const reviewReceipts = receipts.filter((entry) => REVIEW_TRACKED_RECEIPT_KIND_VALUES.includes(String(entry?.receipt_kind || "").trim().toUpperCase()));
  const lastReceipt = receipts.at(-1) || null;
  const lastReviewReceipt = reviewReceipts.at(-1) || null;
  return {
    total,
    reviewTotal: reviewReceipts.length,
    lastReceipt,
    lastReviewReceipt,
  };
}

function formatReceiptKinds(receipts) {
  const counts = new Map();
  for (const entry of receipts || []) {
    const kind = String(entry?.receipt_kind || "UNKNOWN").trim().toUpperCase() || "UNKNOWN";
    counts.set(kind, (counts.get(kind) || 0) + 1);
  }
  return Array.from(counts.entries())
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    .map(([kind, count]) => `- ${kind}: ${count}`);
}

function readValidatorGateSummary(wpId) {
  const gatePath = normalizePath(resolveValidatorGatePath(wpId));
  if (!fs.existsSync(gatePath)) {
    return {
      exists: false,
      gatePath,
    };
  }

  const raw = parseJsonFile(gatePath);
  const session = raw?.validation_sessions?.[wpId] || null;
  const committedEvidence = raw?.committed_validation_evidence?.[wpId] || null;
  const gates = Array.isArray(session?.gates) ? session.gates : [];
  return {
    exists: true,
    gatePath,
    verdict: session?.verdict || "NONE",
    status: session?.status || "NONE",
    gateCount: gates.length,
    lastGate: gates.at(-1) || null,
    committedEvidenceStatus: committedEvidence?.status || "NONE",
    committedEvidenceTarget: committedEvidence?.committed_validation_target || "NONE",
    committedEvidenceHead: committedEvidence?.target_head_sha || "NONE",
    committedEvidenceValidatedAt: committedEvidence?.validated_at || "NONE",
  };
}

function summarizeNotifications(wpId, commDir) {
  const results = checkAllNotifications({ wpId });
  const checks = Object.values(results);
  return {
    checks,
    totalPending: checks.reduce((sum, entry) => sum + Number(entry.pendingCount || 0), 0),
    notificationsPath: normalizePath(path.join(commDir, NOTIFICATIONS_FILE_NAME)),
    cursorPath: normalizePath(path.join(commDir, NOTIFICATION_CURSOR_FILE_NAME)),
  };
}

function formatNotificationLines(notificationSummary) {
  if (!notificationSummary || notificationSummary.checks.length === 0) {
    return ["- NONE"];
  }
  return notificationSummary.checks.map((entry) =>
    `- ${entry.role}: pending=${entry.pendingCount} | by_kind=${Object.entries(entry.byKind).map(([kind, count]) => `${kind}:${count}`).join(", ") || "NONE"}`
  );
}

function communicationMetrics(receipts, controlRequests) {
  const governedReceiptCount = countBy(
    receipts,
    (entry) => ["WP-REVIEW-REQUEST", "WP-REVIEW-RESPONSE", "WP-NOTIFICATION"].includes(String(entry?.receipt_kind || "").trim().toUpperCase()),
  );
  const rawPromptCount = countBy(
    controlRequests,
    (entry) => String(entry?.command_kind || "").trim().toUpperCase() === "SEND_PROMPT",
  );
  const total = governedReceiptCount + rawPromptCount;
  const ratio = total > 0 ? (governedReceiptCount / total) : 0;
  let verdict = "NONE";
  if (governedReceiptCount > 0 && rawPromptCount === 0) verdict = "GOVERNED";
  else if (governedReceiptCount > rawPromptCount) verdict = "MOSTLY_GOVERNED";
  else if (total > 0) verdict = "IMPLICIT";
  return {
    governedReceiptCount,
    rawPromptCount,
    governedRatio: total > 0 ? ratio.toFixed(2) : "0.00",
    communicationVerdict: verdict,
  };
}

function formatSessionLine(session) {
  const commandState = `${session.last_command_kind || "NONE"}/${session.last_command_status || "NONE"}`;
  const host = session.active_host || session.preferred_host || "NONE";
  const thread = session.session_thread_id || "NONE";
  return `- ${session.role} | state=${session.runtime_state} | host=${host} | thread=${thread} | command=${commandState}`;
}

function formatGateLine(entry) {
  const summary = [
    entry.type || "<type>",
    entry.timestamp || "<timestamp>",
    entry.coder_id || entry.execution_lane || "",
    entry.branch || "",
    entry.worktree_dir || "",
  ].filter(Boolean).join(" | ");
  return `- ${summary}`;
}

function buildSkeleton({
  governanceRepoRoot,
  wpId,
  packetPath,
  refinementPath,
  packetText,
  runtimePath,
  receiptsPath,
  threadPath,
  runtime,
  receipts,
  threadSummary,
  gateLogs,
  sessions,
  batchSummary,
  controlRequests,
  controlResults,
  notificationSummary,
  validatorGateSummary,
  communicationStatusEvaluation,
  communicationVerdictEvaluation,
  communicationBoundaryEvaluation,
  computedPolicyEvaluation,
}) {
  const packetStatus = parsePacketStatus(packetText) || "<missing>";
  const boardStatus = taskBoardStatus(governanceRepoRoot, wpId) || "<missing>";
  const dateTag = new Date().toISOString().slice(0, 10).replace(/-/g, "");
  const receiptSummary = formatReceiptSummary(receipts);
  const receiptKindLines = formatReceiptKinds(receipts);
  const sessionLines = sessions.length > 0
    ? sessions.map((session) => formatSessionLine(session))
    : ["- NONE"];
  const gateLines = gateLogs.length > 0
    ? gateLogs.slice(-8).map((entry) => formatGateLine(entry))
    : ["- NONE"];
  const controlRequestLines = controlRequests.length > 0
    ? controlRequests.slice(-5).map((entry) => `- ${entry.command_kind} | ${entry.created_at || "<no-ts>"} | ${entry.role}/${entry.wp_id}`)
    : ["- NONE"];
  const controlResultLines = controlResults.length > 0
    ? controlResults.slice(-5).map((entry) => `- ${entry.command_kind}/${entry.status} | ${entry.processed_at || "<no-ts>"} | ${entry.role}/${entry.wp_id}`)
    : ["- NONE"];
  const notificationLines = formatNotificationLines(notificationSummary);
  const computedPolicyAllowsClosure = computedPolicyOutcomeAllowsClosure(computedPolicyEvaluation);
  const computedPolicyIssueLines = computedPolicyEvaluation?.applicable
    ? [
        ...computedPolicyEvaluation.issues.fail.map((item) => `- FAIL: ${item.code} | ${item.message}`),
        ...computedPolicyEvaluation.issues.blocked.map((item) => `- BLOCKED: ${item.code} | ${item.message}`),
        ...computedPolicyEvaluation.issues.reviewRequired.map((item) => `- REVIEW_REQUIRED: ${item.code} | ${item.message}`),
        ...computedPolicyEvaluation.issues.waived.map((item) => `- WAIVED: ${item.code} | ${item.message} | by=${(item.waived_by || []).join(", ")}`),
      ]
    : ["- NONE"];

  return [
    `# POST_RUN_AUDIT_SKELETON: ${wpId}`,
    "",
    `Generated at: ${new Date().toISOString()}`,
    `Generated by: .GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`,
    "",
    "## Scope",
    `- WP_ID: ${wpId}`,
    `- TASK_BOARD_STATUS: ${boardStatus}`,
    `- PACKET_STATUS: ${packetStatus}`,
    `- PACKET_FORMAT_VERSION: ${parseSingleField(packetText, "PACKET_FORMAT_VERSION") || "<missing>"}`,
    `- WORKFLOW_LANE: ${parseSingleField(packetText, "WORKFLOW_LANE") || "<missing>"}`,
    `- EXECUTION_OWNER: ${parseSingleField(packetText, "EXECUTION_OWNER") || "<missing>"}`,
    `- LOCAL_BRANCH: ${parseSingleField(packetText, "LOCAL_BRANCH") || "<missing>"}`,
    `- LOCAL_WORKTREE_DIR: ${parseSingleField(packetText, "LOCAL_WORKTREE_DIR") || "<missing>"}`,
    "",
    "## Source Artifacts",
    `- PACKET: ${normalizePath(packetPath)}`,
    `- REFINEMENT: ${normalizePath(refinementPath || "<missing>")}`,
    `- RUNTIME_STATUS: ${normalizePath(runtimePath || "<missing>")}`,
    `- RECEIPTS: ${normalizePath(receiptsPath || "<missing>")}`,
    `- THREAD: ${normalizePath(threadPath || "<missing>")}`,
    `- NOTIFICATIONS: ${notificationSummary.notificationsPath}`,
    `- NOTIFICATION_CURSOR: ${notificationSummary.cursorPath}`,
    `- VALIDATOR_GATE_STATE: ${validatorGateSummary.gatePath}`,
    `- ORCHESTRATOR_GATES: ${normalizePath(resolveOrchestratorGatesPath())}`,
    `- SESSION_REGISTRY: ${normalizePath(SESSION_REGISTRY_FILE)}`,
    `- SESSION_CONTROL_REQUESTS: ${normalizePath(SESSION_CONTROL_REQUESTS_FILE)}`,
    `- SESSION_CONTROL_RESULTS: ${normalizePath(SESSION_CONTROL_RESULTS_FILE)}`,
    "",
    "## Runtime Evidence Snapshot",
    `- RUNTIME_STATUS: ${runtime?.runtime_status || "<missing>"}`,
    `- CURRENT_PHASE: ${runtime?.current_phase || "<missing>"}`,
    `- NEXT_EXPECTED_ACTOR: ${runtime?.next_expected_actor || "<missing>"}`,
    `- WAITING_ON: ${runtime?.waiting_on || "<missing>"}`,
    `- READY_FOR_VALIDATION: ${runtime?.ready_for_validation === true ? "YES" : runtime?.ready_for_validation === false ? "NO" : "<missing>"}`,
    `- OPEN_REVIEW_ITEMS: ${Array.isArray(runtime?.open_review_items) ? runtime.open_review_items.length : 0}`,
    `- LAST_EVENT_AT: ${runtime?.last_event_at || "<missing>"}`,
    "",
    "## Receipt / Thread Evidence Snapshot",
    `- TOTAL_RECEIPTS: ${receiptSummary.total}`,
    `- REVIEW_TRACKED_RECEIPTS: ${receiptSummary.reviewTotal}`,
    `- LAST_RECEIPT: ${receiptSummary.lastReceipt ? `${receiptSummary.lastReceipt.receipt_kind} @ ${receiptSummary.lastReceipt.timestamp_utc || "<missing>"}` : "NONE"}`,
    `- LAST_REVIEW_RECEIPT: ${receiptSummary.lastReviewReceipt ? `${receiptSummary.lastReviewReceipt.receipt_kind} @ ${receiptSummary.lastReviewReceipt.timestamp_utc || "<missing>"}` : "NONE"}`,
    `- THREAD_EXISTS: ${threadSummary.exists ? "YES" : "NO"}`,
    `- THREAD_LINE_COUNT: ${threadSummary.lineCount}`,
    `- THREAD_LAST_NONEMPTY_LINE: ${threadSummary.lastNonEmptyLine || "NONE"}`,
    ...receiptKindLines,
    "",
    "## Direct Review / Notification Snapshot",
    `- STATUS_COMMUNICATION_STATE: ${communicationStatusEvaluation?.state || "COMM_NA"}`,
    `- STATUS_COMMUNICATION_OK: ${communicationStatusEvaluation?.ok === true ? "YES" : "NO"}`,
    `- STATUS_COMMUNICATION_MESSAGE: ${communicationStatusEvaluation?.message || "NONE"}`,
    `- VERDICT_COMMUNICATION_STATE: ${communicationVerdictEvaluation?.state || "COMM_NA"}`,
    `- VERDICT_COMMUNICATION_OK: ${communicationVerdictEvaluation?.ok === true ? "YES" : "NO"}`,
    `- VERDICT_BOUNDARY_OK: ${communicationBoundaryEvaluation?.ok === true ? "YES" : "NO"}`,
    `- VERDICT_BOUNDARY_ISSUE_COUNT: ${Array.isArray(communicationBoundaryEvaluation?.issues) ? communicationBoundaryEvaluation.issues.length : 0}`,
    `- PENDING_NOTIFICATION_TOTAL: ${notificationSummary.totalPending}`,
    ...notificationLines,
    "",
    "## Session / Bridge Evidence Snapshot",
    `- LAUNCH_BATCH_MODE: ${batchSummary.launch_batch_mode}`,
    `- LAUNCH_BATCH_PLUGIN_FAILURE_COUNT: ${batchSummary.launch_batch_plugin_failure_count}`,
    `- LAUNCH_BATCH_SWITCHED_AT: ${batchSummary.launch_batch_switched_at || "NONE"}`,
    `- LAUNCH_BATCH_SWITCH_REASON: ${batchSummary.launch_batch_switch_reason || "NONE"}`,
    `- GOVERNED_SESSION_COUNT: ${sessions.length}`,
    ...sessionLines,
    "",
    "## Session Control Evidence Snapshot",
    `- CONTROL_REQUEST_COUNT: ${controlRequests.length}`,
    ...controlRequestLines,
    `- CONTROL_RESULT_COUNT: ${controlResults.length}`,
    ...controlResultLines,
    "",
    "## Orchestrator Gate Evidence Snapshot",
    `- MATCHING_GATE_LOG_COUNT: ${gateLogs.length}`,
    ...gateLines,
    "",
    "## Validator Gate Snapshot",
    `- GATE_FILE_EXISTS: ${validatorGateSummary.exists ? "YES" : "NO"}`,
    `- SESSION_VERDICT: ${validatorGateSummary.verdict || "NONE"}`,
    `- SESSION_STATUS: ${validatorGateSummary.status || "NONE"}`,
    `- GATE_COUNT: ${validatorGateSummary.gateCount || 0}`,
    `- LAST_GATE: ${validatorGateSummary.lastGate ? `${validatorGateSummary.lastGate.gate} @ ${validatorGateSummary.lastGate.timestamp || "<missing>"}` : "NONE"}`,
    `- COMMITTED_EVIDENCE_STATUS: ${validatorGateSummary.committedEvidenceStatus || "NONE"}`,
    `- COMMITTED_EVIDENCE_TARGET: ${validatorGateSummary.committedEvidenceTarget || "NONE"}`,
    `- COMMITTED_EVIDENCE_HEAD: ${validatorGateSummary.committedEvidenceHead || "NONE"}`,
    `- COMMITTED_EVIDENCE_VALIDATED_AT: ${validatorGateSummary.committedEvidenceValidatedAt || "NONE"}`,
    "",
    "## Computed Closure Snapshot",
    `- APPLICABLE: ${computedPolicyEvaluation?.applicable ? "YES" : "NO"}`,
    `- OUTCOME: ${computedPolicyEvaluation?.applicable ? computedPolicyEvaluation.outcome : "NOT_APPLICABLE"}`,
    `- ALLOWS_CLOSURE: ${computedPolicyAllowsClosure ? "YES" : "NO"}`,
    `- FAIL_ISSUES: ${computedPolicyEvaluation?.issues?.fail?.length || 0}`,
    `- BLOCKED_ISSUES: ${computedPolicyEvaluation?.issues?.blocked?.length || 0}`,
    `- REVIEW_REQUIRED_ISSUES: ${computedPolicyEvaluation?.issues?.reviewRequired?.length || 0}`,
    `- WAIVED_ISSUES: ${computedPolicyEvaluation?.issues?.waived?.length || 0}`,
    ...computedPolicyIssueLines,
    "",
    "## Timeline Skeleton",
    "- PREPARE:",
    "- STARTUP / SESSION LAUNCH:",
    "- IMPLEMENTATION / REVIEW ROUTING:",
    "- VALIDATION / VERDICT:",
    "- CLOSEOUT / SYNC:",
    "",
    "## Structured Failure Ledger",
    `### 5.1 ${wpId} finding placeholder`,
    `- FINDING_ID: SMOKE-FIND-${dateTag}-01`,
    "- CATEGORY: WORKFLOW_DISCIPLINE",
    "- ROLE_OWNER: SHARED",
    "- SYSTEM_SCOPE: CONTROL_PLANE",
    "- FAILURE_CLASS: UX_AMBIGUITY",
    "- SURFACE:",
    "- SEVERITY: MEDIUM",
    "- STATUS: OPEN",
    "- RELATED_GOVERNANCE_ITEMS:",
    "  - NONE",
    "- REGRESSION_HOOKS:",
    "  - just gov-check",
    "- Evidence:",
    "  - NONE",
    "- What went wrong:",
    "  - NONE",
    "- Impact:",
    "  - NONE",
    "- Mechanical fix direction:",
    "  - NONE",
    "",
    "## Role Review Seed",
    "### 6.1 Orchestrator Review",
    "- Strengths:",
    "  - NONE",
    "- Failures:",
    "  - NONE",
    "- Assessment:",
    "  - NONE",
    "",
    "### 6.2 Coder Review",
    "- Strengths:",
    "  - NONE",
    "- Failures:",
    "  - NONE",
    "- Assessment:",
    "  - NONE",
    "",
    "### 6.3 WP Validator Review",
    "- Strengths:",
    "  - NONE",
    "- Failures:",
    "  - NONE",
    "- Assessment:",
    "  - NONE",
    "",
    "### 6.4 Integration Validator Review",
    "- Strengths:",
    "  - NONE",
    "- Failures:",
    "  - NONE",
    "- Assessment:",
    "  - NONE",
    "",
    "## Failure Classification Summary",
    "- ROLE_FAILURE_COUNTS:",
    "  - ORCHESTRATOR: 0",
    "  - CODER: 0",
    "  - WP_VALIDATOR: 0",
    "  - INTEGRATION_VALIDATOR: 0",
    "  - OPERATOR: 0",
    "  - SHARED: 0",
    "- SYSTEMIC_FAILURE_COUNTS:",
    "  - CONTROL_PLANE: 0",
    "  - CROSS_ROLE: 0",
    "  - LOCAL: 0",
    "- FAILURE_CLASS_COUNTS:",
    "  - CHECK_FAILURE: 0",
    "  - SCRIPT_DEFECT: 0",
    "  - RUNTIME_TRUTH: 0",
    "  - STATUS_DRIFT: 0",
    "  - OUT_OF_SCOPE: 0",
    "  - STALL: 0",
    "  - COMMAND_SURFACE_MISUSE: 0",
    "  - UX_AMBIGUITY: 0",
    "  - TOKEN_WASTE: 0",
    "  - OTHER: 0",
    "",
    "## Governance Linkage and Board Mapping",
    "- BOARD_LINKS:",
    "  - NONE",
    "- CHANGESET_LINKS:",
    "  - NONE",
    "- POLICY_OR_TEMPLATE_FOLLOWUPS:",
    "  - NONE",
    "",
    "## Positive Controls Worth Preserving",
    `### 10.1 ${wpId} positive control placeholder`,
    `- CONTROL_ID: SMOKE-CONTROL-${dateTag}-01`,
    "- CONTROL_TYPE: REGRESSION_GUARD",
    "- SURFACE:",
    "- What went well:",
    "  - NONE",
    "- Why it mattered:",
    "  - NONE",
    "- Evidence:",
    "  - NONE",
    "- REGRESSION_GUARDS:",
    "  - just gov-check",
    "",
    "## Proof Gaps / NOT_PROVEN",
    "- ITEM_1:",
    "",
    "## Follow-up Checks",
    "- CHECK_1:",
    "",
  ].join("\n");
}

function buildLiveReview({
  governanceRepoRoot,
  now,
  wpId,
  packetPath,
  refinementPath,
  packetAbsPath,
  packetText,
  runtimePath,
  receiptsPath,
  threadPath,
  runtime,
  receipts,
  threadSummary,
  sessions,
  controlRequests,
  controlResults,
  notificationSummary,
  brokerSummary,
  currentSession,
}) {
  const dateTag = formatDateTag(now);
  const slugUnderscore = auditSlugFromWpId(wpId);
  const slugHyphen = auditSlugHyphen(wpId);
  const packetStatus = parsePacketStatus(packetText) || "<missing>";
  const boardStatus = taskBoardStatus(governanceRepoRoot, wpId) || "<missing>";
  const workflowLane = parseSingleField(packetText, "WORKFLOW_LANE") || "<missing>";
  const executionOwner = parseSingleField(packetText, "EXECUTION_OWNER") || "<missing>";
  const microtaskRows = buildPerMicrotaskSeedRows(listMicrotasks(packetAbsPath));
  const metrics = communicationMetrics(receipts, controlRequests);
  const brokerRunLines = formatBrokerActiveRuns(brokerSummary.activeRuns);
  const sessionLines = sessions.length > 0
    ? sessions.map((session) => formatSessionLine(session))
    : ["- NONE"];
  const commandLines = controlResults.length > 0
    ? controlResults.slice(-8).map((entry) => `- ${entry.command_kind}/${entry.status} | ${entry.processed_at || "<no-ts>"} | ${entry.role}/${entry.wp_id}`)
    : ["- NONE"];
  const receiptKindLines = formatReceiptKinds(receipts);
  const notificationLines = formatNotificationLines(notificationSummary);
  const sessionIntention = String(
    currentSession?.topic
    || ""
  ).trim() || "<fill from repomem session topic>";
  const currentStatusLine = runtime
    ? `Current packet/runtime status is ${packetStatus} / ${runtime?.runtime_status || "<missing>"} with next actor ${runtime?.next_expected_actor || "<missing>"}.`
    : "Runtime status file is missing or not yet initialized.";

  return [
    `# DOSSIER_${dateTag}_${slugUnderscore}_WORKFLOW_DOSSIER`,
    "",
    "## METADATA",
    "",
    `- WORKFLOW_DOSSIER_ID: WORKFLOW-DOSSIER-${dateTag}-${slugHyphen}`,
    `- AUDIT_ID: AUDIT-${dateTag}-${slugHyphen}-SMOKETEST-REVIEW`,
    `- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-${dateTag}-${slugHyphen}`,
    "- DOCUMENT_KIND: LIVE_WORKFLOW_DOSSIER",
    "- LIVE_REVIEW_STATUS: OPEN",
    `- REPO_TIMEZONE: ${REPORT_TIMEZONE}`,
    "- REVIEW_KIND: <SET_AT_CLOSEOUT>",
    `- DATE_LOCAL: ${formatLocalDate(now)}`,
    `- DATE_UTC: ${now.toISOString().slice(0, 10)}`,
    `- OPENED_AT_LOCAL: ${formatLocalTimestamp(now)}`,
    `- OPENED_AT_UTC: ${now.toISOString()}`,
    `- LAST_UPDATED_LOCAL: ${formatLocalTimestamp(now)}`,
    `- LAST_UPDATED_UTC: ${now.toISOString()}`,
    `- SESSION_INTENTION: ${sessionIntention}`,
    "- AUTHOR: Codex acting as ORCHESTRATOR",
    "- HISTORICAL_BASELINE_PACKET: NONE",
    `- ACTIVE_RECOVERY_PACKET: ${wpId}`,
    "- LINEAGE_STATUS: NONE",
    "- RELATED_PREVIOUS_REVIEWS:",
    "  - NONE",
    "- SCOPE:",
    `  - live workflow dossier opened at WP activation for \`${packetPath}\``,
    `  - workflow lane \`${workflowLane}\` with execution owner \`${executionOwner}\``,
    `  - ACP/session-control/runtime surfaces under \`../gov_runtime\``,
    "- RESULT:",
    "  - PRODUCT_REMEDIATION: PARTIAL",
    "  - MASTER_SPEC_AUDIT: PARTIAL",
    "  - WORKFLOW_DISCIPLINE: PARTIAL",
    "  - ACP_RUNTIME_DISCIPLINE: PARTIAL",
    "  - MERGE_PROGRESSION: PARTIAL",
    "- KEY_COMMITS_REVIEWED:",
    "  - NONE yet",
    "- EVIDENCE_SOURCES:",
    `  - \`${packetPath}\``,
    `  - \`${normalizePath(refinementPath || "NONE")}\``,
    `  - \`${normalizePath(runtimePath || "NONE")}\``,
    `  - \`${normalizePath(receiptsPath || "NONE")}\``,
    `  - \`${normalizePath(threadPath || "NONE")}\``,
    `  - \`${normalizePath(SESSION_CONTROL_REQUESTS_FILE)}\``,
    `  - \`${normalizePath(SESSION_CONTROL_RESULTS_FILE)}\``,
    `  - \`${normalizePath(SESSION_CONTROL_OUTPUT_DIR)}\``,
    `  - \`${normalizePath(SESSION_REGISTRY_FILE)}\``,
    `  - \`${brokerSummary.path}\``,
    "- RELATED_GOVERNANCE_ITEMS:",
    "  - NONE",
    "- RELATED_CHANGESETS:",
    "  - NONE",
    "",
    "---",
    "",
    "## 1. Executive Summary",
    "",
    "- LIVE REVIEW OPENED at activation. This document is the run-time workflow dossier for the WP and should be updated as the run progresses.",
    `- ${currentStatusLine}`,
    "",
    "## 2. Lineage and What This Run Needed To Prove",
    "",
    "- This review was opened at packet activation instead of reconstructed at closeout.",
    "- Fill this section with the specific product and workflow truths the run needs to prove.",
    "",
    "### What Improved vs Previous Smoketest",
    "",
    "- NONE yet — live review opened at activation.",
    "",
    "## 3. Product Outcome",
    "",
    "- NONE yet — fill as product work lands.",
    "",
    "## 4. Timeline",
    "",
    "| Time (Europe/Brussels) | Event |",
    "|---|---|",
    `| ${formatLocalTimestamp(now)} | Live workflow dossier created at WP activation |`,
    `| ${formatDisplayTimeFromIso(runtime?.last_event_at)} | Latest runtime event at creation time |`,
    "",
    "## 5. Per-Microtask Breakdown",
    "",
    "| MT | Prompt Summary | Commit | Time Sent | Time Committed | Compile First Pass | Validator Flagged | Fix Cycles |",
    "|---|---|---|---|---|---|---|---|",
    ...microtaskRows,
    "",
    "## 6. Communication Trail Audit",
    "",
    "List every inter-role message with timestamps and communication surface used as the run progresses.",
    "",
    "| # | Time | From | To | Surface | Content Summary |",
    "|---|---|---|---|---|---|",
    "| 1 | <fill> | <fill> | <fill> | <fill> | <fill> |",
    "",
    "Assessment:",
    `- GOVERNED_RECEIPT_COUNT: ${metrics.governedReceiptCount}`,
    `- RAW_PROMPT_COUNT: ${metrics.rawPromptCount}`,
    `- GOVERNED_RATIO: ${metrics.governedRatio}`,
    `- COMMUNICATION_VERDICT: ${metrics.communicationVerdict}`,
    "",
    "## 7. Structured Failure Ledger",
    "",
    `### 7.1 ${wpId} finding placeholder`,
    `- FINDING_ID: SMOKE-FIND-${dateTag}-01`,
    "- CATEGORY: WORKFLOW_DISCIPLINE",
    "- ROLE_OWNER: SHARED",
    "- SYSTEM_SCOPE: CONTROL_PLANE",
    "- FAILURE_CLASS: UX_AMBIGUITY",
    "- SURFACE:",
    "- SEVERITY: MEDIUM",
    "- STATUS: OPEN",
    "- RELATED_GOVERNANCE_ITEMS:",
    "  - NONE",
    "- REGRESSION_HOOKS:",
    "  - just gov-check",
    "- Evidence:",
    "  - NONE",
    "- What went wrong:",
    "  - NONE yet",
    "- Impact:",
    "  - NONE yet",
    "- Mechanical fix direction:",
    "  - NONE yet",
    "",
    "## 8. Role Review",
    "",
    "### 8.1 Orchestrator Review",
    "",
    "Strengths:",
    "",
    "- NONE yet",
    "",
    "Failures:",
    "",
    "- NONE yet",
    "",
    "Assessment:",
    "",
    "- NONE yet",
    "",
    "### 8.2 Coder Review",
    "",
    "Strengths:",
    "",
    "- NONE yet",
    "",
    "Failures:",
    "",
    "- NONE yet",
    "",
    "Assessment:",
    "",
    "- NONE yet",
    "",
    "### 8.3 WP Validator Review",
    "",
    "Strengths:",
    "",
    "- NONE yet",
    "",
    "Failures:",
    "",
    "- NONE yet",
    "",
    "Assessment:",
    "",
    "- NONE yet",
    "",
    "### 8.4 Integration Validator Review",
    "",
    "Strengths:",
    "",
    "- NONE yet",
    "",
    "Failures:",
    "",
    "- NONE yet",
    "",
    "Assessment:",
    "",
    "- NONE yet",
    "",
    "## 9. Review Of Coder and Validator Communication",
    "",
    "- NONE yet — fill as direct review traffic appears.",
    "",
    "## 9a. Memory Discipline",
    "",
    "- MEMORY_WRITES_BY_ROLE:",
    "  - ORCHESTRATOR: NONE",
    "  - CODER: NONE",
    "  - WP_VALIDATOR: NONE",
    "  - INTEGRATION_VALIDATOR: NONE",
    "- MEMORY_WRITE_EVIDENCE:",
    "  - NONE",
    "- DUAL_WRITE_COMPLIANCE: PARTIAL",
    "- MEMORY_VERDICT: NONE",
    "- Assessment:",
    "  - NONE yet",
    "",
    "## 9b. Build Artifact Hygiene",
    "",
    "- BUILD_TARGET_PATH: `<WORKSPACE_ROOT>/Handshake Artifacts`",
    "- BUILD_TARGET_CLEANED_BY: NONE",
    "- BUILD_TARGET_CLEANED_AT: N/A",
    "- BUILD_TARGET_STATE_AT_CLOSEOUT: NOT_CHECKED",
    "- Assessment:",
    "  - NONE yet",
    "",
    "## 10. ACP Runtime / Session Control Findings",
    "",
    `- BROKER_STATE_FILE: \`${brokerSummary.path}\``,
    `- SESSION_CONTROL_OUTPUT_DIR: \`${brokerSummary.outputDir}\``,
    `- BROKER_PRESENT: ${brokerSummary.exists ? "YES" : "NO"}`,
    `- BROKER_BUILD_ID: ${brokerSummary.buildId}`,
    `- BROKER_AUTH_MODE: ${brokerSummary.authMode}`,
    `- BROKER_HOST: ${brokerSummary.host}:${brokerSummary.port || 0}`,
    `- BROKER_PID: ${brokerSummary.brokerPid || 0}`,
    `- BROKER_UPDATED_AT_UTC: ${brokerSummary.updatedAt}`,
    `- BROKER_ACTIVE_RUN_COUNT: ${brokerSummary.activeRuns.length}`,
    `- GOVERNED_SESSION_COUNT: ${sessions.length}`,
    `- CONTROL_REQUEST_COUNT: ${controlRequests.length}`,
    `- CONTROL_RESULT_COUNT: ${controlResults.length}`,
    `- PENDING_NOTIFICATION_TOTAL: ${notificationSummary.totalPending}`,
    "",
    "Active runs:",
    ...brokerRunLines,
    "",
    "Governed sessions:",
    ...sessionLines,
    "",
    "Latest control results:",
    ...commandLines,
    "",
    "Receipt kinds:",
    ...receiptKindLines,
    "",
    "Notification state:",
    ...notificationLines,
    "",
    "## 11. Terminal Hygiene",
    "",
    "- TERMINALS_LAUNCHED: <fill>",
    "- TERMINALS_CLOSED_ON_COMPLETION: <fill>",
    "- TERMINALS_CLOSED_ON_FAILURE: <fill>",
    "- TERMINALS_RECLAIMED_AT_CLOSEOUT: <fill>",
    "- STALE_BLANK_TERMINALS_REMAINING: <fill>",
    "- TERMINAL_HYGIENE_VERDICT: <CLEAN|PARTIAL|FAILED>",
    "",
    "Assessment:",
    "",
    "- NONE yet",
    "",
    "## 12. Governance Linkage and Board Mapping",
    "",
    "- BOARD_LINKS:",
    "  - NONE",
    "- CHANGESET_LINKS:",
    "  - NONE",
    "- POLICY_OR_TEMPLATE_FOLLOWUPS:",
    "  - NONE yet",
    "",
    "## 13. Positive Controls Worth Preserving",
    "",
    `### 13.1 ${wpId} positive control placeholder`,
    `- CONTROL_ID: SMOKE-CONTROL-${dateTag}-01`,
    "- CONTROL_TYPE: REGRESSION_GUARD",
    "- SURFACE:",
    "- What went well:",
    "  - NONE yet",
    "- Why it mattered:",
    "  - NONE yet",
    "- Evidence:",
    "  - NONE yet",
    "- REGRESSION_GUARDS:",
    "  - just gov-check",
    "",
    "## 14. Cost Attribution",
    "",
    "| Phase | Time (min) | Orchestrator Tokens (est) | Notes |",
    "|---|---|---|---|",
    "| Refinement | <N> | <N or %> | |",
    "| Per-MT Coding (total) | <N> | <N or %> | |",
    "| Validation | <N> | <N or %> | |",
    "| Fix Cycle | <N> | <N or %> | |",
    "| Closeout | <N> | <N or %> | |",
    "| Polling/Waiting | <N> | <N or %> | |",
    "| TOTAL | <N> | <N or %> | |",
    "",
    "## 15. Comparison Table (vs Previous WP)",
    "",
    "| Metric | Previous WP | This WP | Trend |",
    "|---|---|---|---|",
    "| Total lines changed | <N> | <N> | |",
    "| Microtask count | <N> | <N> | |",
    "| Compile errors (first pass) | <N> | <N> | |",
    "| Validator findings | <N> | <N> | |",
    "| Fix cycles | <N> | <N> | |",
    "| Stubs discovered | <N> | <N> | |",
    "| Governed receipts created | <N> | <N> | |",
    "",
    "## Workflow Dossier Closeout Rubric",
    "",
    "- Fill at closeout using `.GOV/roles_shared/docs/WORKFLOW_DOSSIER_RUBRIC.md`.",
    "",
    "## 17. Silent Failures, Command Surface Misuse, and Ambiguity Scan",
    "",
    "- Fill at closeout using `.GOV/roles_shared/docs/WORKFLOW_DOSSIER_RUBRIC.md`.",
    "",
    "## 18. What Should Change Before The Next Run",
    "",
    "- NONE yet",
    "",
    "## 19. Suggested Remediations",
    "",
    "### Governance / Runtime",
    "",
    "- NONE yet",
    "",
    "### Product / Validation Quality",
    "",
    "- NONE yet",
    "",
    "### Documentation / Review Practice",
    "",
    "- NONE yet",
    "",
    "## 20. Command Log",
    "",
    "- `just orchestrator-prepare-and-packet` -> PASS (live workflow dossier created during activation)",
    "",
    "## LIVE_EXECUTION_LOG (append-only during WP execution)",
    "",
    "This section is append-only. The Orchestrator records execution milestones, dead-time observations, ACP/runtime events, and route changes as they happen.",
    "",
    "Format: `- [TIMESTAMP] [ROLE] [TYPE] [SURFACE] <summary>`",
    "",
    `- [${formatLocalTimestamp(now)}] [ORCHESTRATOR] [REVIEW_OPENED] [${normalizePath(packetPath)}] Live workflow dossier created with current ACP/session snapshot`,
    "",
    "## LIVE_IDLE_LEDGER (append-only during WP execution)",
    "",
    "This section is append-only. Mechanical sync appends latency, idle-gap, and drift markers derived from ACP/session-control plus WP communication timing.",
    "",
    "Format: `- [TIMESTAMP] [ROLE] [IDLE_LEDGER] [SURFACE] <mechanical summary>`",
    "",
    "- [<TIMESTAMP>] [ORCHESTRATOR] [IDLE_LEDGER] [MECHANICAL] <review_rtt|pass_to_coder|idle|drift>",
    "",
    "## LIVE_GOVERNANCE_CHANGE_LOG (append-only during WP execution)",
    "",
    "This section is append-only. Record governance-only refactors, template changes, helper patches, and protocol repairs made during the run.",
    "",
    "Format: `- [TIMESTAMP] [ROLE] [CHANGE_TYPE] <surface> :: <summary>`",
    "",
    "- [<TIMESTAMP>] [ORCHESTRATOR] [PATCH] <surface> :: <summary>",
    "",
    "## LIVE_CONCERNS_LOG (append-only during WP execution)",
    "",
    "This section is append-only. Capture unresolved concerns, skepticism, or operator-observed smells before closeout.",
    "",
    "Format: `- [TIMESTAMP] [ROLE] [CONCERN] <summary>`",
    "",
    "- [<TIMESTAMP>] [ORCHESTRATOR] [CONCERN] <summary>",
    "",
    "## LIVE_FINDINGS_LOG (append-only during WP execution)",
    "",
    "This section is append-only. Roles add findings as they occur during WP work.",
    "",
    "Format: `- [TIMESTAMP] [ROLE] [CATEGORY] <finding>`",
    "",
    "- [<TIMESTAMP>] [CODER|WP_VALIDATOR|ORCHESTRATOR] [CATEGORY] <finding>",
  ].join("\n");
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const now = new Date();
  const resolvedPacket = resolveWorkPacketPath(options.wpId);
  if (!resolvedPacket?.packetPath || !fs.existsSync(resolvedPacket.packetAbsPath || "")) {
    fail(`Packet not found for ${options.wpId}`);
  }

  const packetPath = resolvedPacket.packetPath;
  const packetAbsPath = resolvedPacket.packetAbsPath;
  const governanceRepoRoot = path.resolve(path.dirname(packetAbsPath), "..", "..", "..");
  const repoRoot = governanceRepoRoot;
  const refinementPath = resolveRefinementPath(options.wpId) || "";
  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  const runtimePath = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsPath = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const threadPath = parseSingleField(packetText, "WP_THREAD_FILE");
  const commDir = parseSingleField(packetText, "WP_COMMUNICATION_DIR");
  const runtime = runtimePath && fs.existsSync(repoPathAbs(runtimePath)) ? parseJsonFile(runtimePath) : null;
  const receipts = receiptsPath && fs.existsSync(repoPathAbs(receiptsPath)) ? parseJsonlFile(receiptsPath) : [];
  const threadSummary = readThreadSummary(threadPath);
  const notificationSummary = summarizeNotifications(options.wpId, commDir);
  const brokerSummary = readBrokerState(repoRoot);
  const currentSession = getCurrentSession() || {};
  if (options.sessionIntention) {
    currentSession.topic = options.sessionIntention;
  }
  const latestReceipt = receipts.at(-1) || null;
  const pendingNotifications = notificationSummary.checks.flatMap((entry) => entry.notifications || []);
  const communicationHealthArgs = {
    wpId: options.wpId,
    packetPath,
    packetContent: packetText,
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE"),
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    receipts,
    runtimeStatus: runtime || {},
  };
  const communicationStatusEvaluation = evaluateWpCommunicationHealth({
    ...communicationHealthArgs,
    stage: "STATUS",
  });
  const communicationVerdictEvaluation = evaluateWpCommunicationHealth({
    ...communicationHealthArgs,
    stage: "VERDICT",
  });
  const communicationBoundaryEvaluation = evaluateWpCommunicationBoundary({
    stage: "VERDICT",
    statusEvaluation: communicationVerdictEvaluation,
    runtimeStatus: runtime || {},
    latestReceipt,
    pendingNotifications,
  });
  const computedPolicyEvaluation = evaluateComputedPolicyGateFromPacketText(packetText, {
    wpId: options.wpId,
    packetPath,
    requireClosedStatus: true,
  });
  const validatorGateSummary = readValidatorGateSummary(options.wpId);
  const orchestratorGatesPath = resolveOrchestratorGatesPath();
  const gateState = fs.existsSync(orchestratorGatesPath) ? parseJsonFile(orchestratorGatesPath) : {};
  const gateLogs = Array.isArray(gateState.gate_logs)
    ? gateState.gate_logs.filter((entry) => String(entry?.wpId || "").trim() === options.wpId)
    : [];

  const { registry } = loadSessionRegistry(repoRoot);
  const sessions = (registry.sessions || [])
    .filter((session) => String(session.wp_id || "").trim() === options.wpId)
    .map((session) => registrySessionSummary(session));
  const batchSummary = registryBatchLaunchSummary(registry);
  const { requests: controlRequestsAll } = loadSessionControlRequests(repoRoot);
  const { results: controlResultsAll } = loadSessionControlResults(repoRoot);
  const controlRequests = controlRequestsAll.filter((entry) => String(entry.wp_id || "").trim() === options.wpId);
  const controlResults = controlResultsAll.filter((entry) => String(entry.wp_id || "").trim() === options.wpId);

  const content = options.mode === "live"
    ? buildLiveReview({
        governanceRepoRoot,
        now,
        wpId: options.wpId,
        packetPath,
        refinementPath,
        packetAbsPath,
        packetText,
        runtimePath,
        receiptsPath,
        threadPath,
        runtime,
        receipts,
        threadSummary,
        sessions,
        controlRequests,
        controlResults,
        notificationSummary,
        brokerSummary,
        currentSession,
      })
    : buildSkeleton({
        governanceRepoRoot,
        wpId: options.wpId,
        packetPath,
        refinementPath,
        packetText,
        runtimePath,
        receiptsPath,
        threadPath,
        runtime,
        receipts,
        threadSummary,
        gateLogs,
        sessions,
        batchSummary,
        controlRequests,
        controlResults,
        notificationSummary,
        validatorGateSummary,
        communicationStatusEvaluation,
        communicationVerdictEvaluation,
        communicationBoundaryEvaluation,
        computedPolicyEvaluation,
      });

  const existingLiveRelativePath = options.mode === "live"
    ? findExistingLiveReviewRelativePath(governanceRepoRoot, options.wpId)
    : "";
  const outputRelativePath = options.output
    ? normalizePath(options.output)
    : options.autoOutput
      ? (existingLiveRelativePath || defaultLiveReviewRelativePath(governanceRepoRoot, options.wpId, now))
      : "";

  if (outputRelativePath) {
    const outputPath = path.resolve(repoRoot, outputRelativePath);
    if (options.mode === "live" && fs.existsSync(outputPath) && !options.force) {
      console.log(normalizePath(path.relative(repoRoot, outputPath)) || normalizePath(outputPath));
      return;
    }
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(outputPath, `${content}\n`, "utf8");
    console.log(normalizePath(path.relative(repoRoot, outputPath)) || normalizePath(outputPath));
    return;
  }

  process.stdout.write(`${content}\n`);
}

main();
