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
import { checkAllNotifications } from "../wp/wp-check-notifications.mjs";

function fail(message) {
  console.error(`[POST_RUN_AUDIT_SKELETON] ${message}`);
  process.exit(1);
}

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function parseArgs(argv) {
  const args = [...argv];
  const options = {
    wpId: "",
    output: "",
  };

  while (args.length > 0) {
    const token = String(args.shift() || "").trim();
    if (!token) continue;
    if (!options.wpId && /^WP-/.test(token)) {
      options.wpId = token;
      continue;
    }
    if (token === "--output") {
      options.output = String(args.shift() || "").trim();
      continue;
    }
    fail(`Unknown argument: ${token}`);
  }

  if (!options.wpId) {
    fail("Usage: node .GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs WP-{ID} [--output <file.md>]");
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
    "## Findings",
    "- FINDING_1:",
    "",
    "## Concerns / Possible Workflow Failures",
    "- CODER:",
    "- WP_VALIDATOR:",
    "- INTEGRATION_VALIDATOR:",
    "- ORCHESTRATOR:",
    "- GOVERNANCE / TOOLING:",
    "",
    "## Proof Gaps / NOT_PROVEN",
    "- ITEM_1:",
    "",
    "## Follow-up Checks",
    "- CHECK_1:",
    "",
  ].join("\n");
}

function main() {
  const options = parseArgs(process.argv.slice(2));
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

  const content = buildSkeleton({
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

  if (options.output) {
    const outputPath = path.resolve(repoRoot, options.output);
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(outputPath, `${content}\n`, "utf8");
    console.log(normalizePath(path.relative(repoRoot, outputPath)) || normalizePath(outputPath));
    return;
  }

  process.stdout.write(`${content}\n`);
}

main();
