#!/usr/bin/env node
/**
 * WP Lane Health Check — single-command diagnostic for active WP execution.
 *
 * Checks:
 * 1. Session states (READY/RUNNING/FAILED/COMPLETED) for coder + validator
 * 2. Hook installation (post-commit hook exists in coder worktree)
 * 3. Notification queue (pending undelivered notifications)
 * 4. Stall detection (time since last receipt/notification progress)
 * 5. Commit progress (how many MT commits exist on the feature branch)
 * 6. Auto-relay readiness (validator session alive and steerable)
 * 7. Broker responsiveness (can we reach the broker)
 *
 * Usage: node wp-lane-health.mjs <WP_ID>
 */

import fs from "node:fs";
import path from "node:path";
import { execFileSync, execSync } from "node:child_process";
import { loadSessionRegistry } from "./session-registry-lib.mjs";
import { sessionKey, defaultCoderBranch, defaultCoderWorktreeDir, SESSION_CONTROL_BROKER_STATE_FILE } from "./session-policy.mjs";
import { REPO_ROOT, normalizePath, repoPathAbs, resolveWorkPacketPath } from "../lib/runtime-paths.mjs";
import { parseJsonFile, parseJsonlFile } from "../lib/wp-communications-lib.mjs";
import { readExecutionProjectionView } from "../lib/wp-execution-state-lib.mjs";
import {
  normalizeRelayEscalationPolicy,
  relayEscalationPolicyBudgetLabel,
} from "../lib/wp-relay-policy-lib.mjs";
import { readWpTokenUsageLedger } from "./wp-token-usage-lib.mjs";
import { evaluateWpTokenBudget } from "./wp-token-budget-lib.mjs";
import {
  activeRunsForSession,
  buildSessionTelemetry,
  formatPushAlertInline,
  formatSessionRunTelemetryInline,
  formatSessionStepTelemetryInline,
  selectLatestPushAlert,
} from "./session-telemetry-lib.mjs";

const wpId = String(process.argv[2] || "").trim();
if (!wpId || !wpId.startsWith("WP-")) {
  console.error("Usage: node wp-lane-health.mjs <WP_ID>");
  process.exit(1);
}

const issues = [];
const info = [];

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function displayRepoRelativePath(value) {
  const raw = String(value || "").trim();
  if (!raw) return "";
  const absolutePath = path.isAbsolute(raw) ? path.resolve(raw) : path.resolve(REPO_ROOT, raw);
  return normalizePath(path.relative(REPO_ROOT, absolutePath) || ".");
}

function loadRuntimeStatusForWp(id) {
  try {
    const packetPath = resolveWorkPacketPath(id);
    const packetAbsPath = repoPathAbs(packetPath);
    if (!packetPath || !fs.existsSync(packetAbsPath)) return null;
    const packetText = fs.readFileSync(packetAbsPath, "utf8");
    const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
    if (!runtimeStatusFile) return null;
    const runtimeStatusAbs = repoPathAbs(runtimeStatusFile);
    if (!fs.existsSync(runtimeStatusAbs)) return null;
    return readExecutionProjectionView(parseJsonFile(runtimeStatusFile)).runtime || null;
  } catch {
    return null;
  }
}

// 1. Session states
const { registry } = loadSessionRegistry(REPO_ROOT);
const coderKey = sessionKey("CODER", wpId);
const validatorKey = sessionKey("WP_VALIDATOR", wpId);
const coderSession = registry.sessions.find((s) => s.session_key === coderKey);
const validatorSession = registry.sessions.find((s) => s.session_key === validatorKey);
const runtimeStatus = loadRuntimeStatusForWp(wpId);
const relayPolicy = normalizeRelayEscalationPolicy(runtimeStatus?.relay_escalation_policy);
const brokerActiveRuns = fs.existsSync(repoPathAbs(SESSION_CONTROL_BROKER_STATE_FILE))
  ? (parseJsonFile(SESSION_CONTROL_BROKER_STATE_FILE)?.active_runs || [])
  : [];

if (!coderSession) {
  issues.push("CODER session not registered");
} else {
  info.push(`CODER: ${coderSession.runtime_state}`);
  info.push(`CODER health: ${coderSession.health_state || "UNKNOWN"} (${coderSession.health_reason_code || "UNKNOWN"})`);
  {
    const telemetry = buildSessionTelemetry({
      session: coderSession,
      activeRuns: activeRunsForSession(coderSession, brokerActiveRuns),
      repoRoot: REPO_ROOT,
    });
    info.push(`CODER telemetry: ${formatSessionRunTelemetryInline(telemetry.run)} | ${formatSessionStepTelemetryInline(telemetry.step)}`);
  }
  if (coderSession.runtime_state === "FAILED") issues.push("CODER session FAILED — needs restart");
  if (coderSession.runtime_state === "COMPLETED") issues.push("CODER session COMPLETED — may need restart for more MTs");
}

if (!validatorSession) {
  issues.push("WP_VALIDATOR session not registered");
} else {
  info.push(`WP_VALIDATOR: ${validatorSession.runtime_state}`);
  info.push(`WP_VALIDATOR health: ${validatorSession.health_state || "UNKNOWN"} (${validatorSession.health_reason_code || "UNKNOWN"})`);
  {
    const telemetry = buildSessionTelemetry({
      session: validatorSession,
      activeRuns: activeRunsForSession(validatorSession, brokerActiveRuns),
      repoRoot: REPO_ROOT,
    });
    info.push(`WP_VALIDATOR telemetry: ${formatSessionRunTelemetryInline(telemetry.run)} | ${formatSessionStepTelemetryInline(telemetry.step)}`);
  }
  if (validatorSession.runtime_state === "FAILED") issues.push("WP_VALIDATOR session FAILED — auto-relay will not work");
  if (validatorSession.runtime_state === "COMPLETED") issues.push("WP_VALIDATOR session COMPLETED — auto-relay will not work");
  if (!["READY", "COMMAND_RUNNING"].includes(validatorSession.runtime_state)) {
    issues.push(`WP_VALIDATOR not steerable (state: ${validatorSession.runtime_state}) — auto-relay dispatch will fail`);
  }
}

// 2. Hook installation
const worktreeDir = repoPathAbs(defaultCoderWorktreeDir(wpId));
if (fs.existsSync(worktreeDir)) {
  const dotGitPath = path.join(worktreeDir, ".git");
  let hookExists = false;
  try {
    if (fs.statSync(dotGitPath).isFile()) {
      const gitdirContent = fs.readFileSync(dotGitPath, "utf8").trim();
      const match = gitdirContent.match(/^gitdir:\s*(.+)$/);
      if (match) {
        const realGitDir = path.isAbsolute(match[1]) ? match[1] : path.resolve(worktreeDir, match[1]);
        const hookPath = path.join(realGitDir, "hooks", "post-commit");
        hookExists = fs.existsSync(hookPath);
      }
    } else {
      hookExists = fs.existsSync(path.join(dotGitPath, "hooks", "post-commit"));
    }
  } catch {}
  if (hookExists) {
    info.push("MT hook: INSTALLED");
  } else {
    issues.push("MT hook: NOT INSTALLED — auto-relay will not fire on commits. Run: just install-mt-hook " + wpId);
  }
} else {
  issues.push(`Coder worktree not found: ${displayRepoRelativePath(worktreeDir)}`);
}

// 3. Commit progress
if (fs.existsSync(worktreeDir)) {
  try {
    const branch = defaultCoderBranch(wpId);
    const log = execSync(`git -C "${worktreeDir}" log --oneline --grep="^feat: MT-" --format="%s"`, { encoding: "utf8" }).trim();
    const mtCommits = log ? log.split("\n").filter(Boolean) : [];
    info.push(`MT commits: ${mtCommits.length}`);
    for (const msg of mtCommits) info.push(`  ${msg}`);
  } catch {}
}

// 4. Notification queue
try {
  const commDir = path.join(REPO_ROOT, "..", "gov_runtime", "roles_shared", "WP_COMMUNICATIONS", wpId);
  const notificationsFile = path.join(commDir, "NOTIFICATIONS.jsonl");
  if (fs.existsSync(notificationsFile)) {
    const notifications = parseJsonlFile(notificationsFile);
    const pending = notifications.filter((n) => !n.acknowledged);
    const latestPushAlert = selectLatestPushAlert(pending);
    if (pending.length > 0) {
      issues.push(`${pending.length} unacknowledged notification(s) — auto-relay may have failed`);
      for (const n of pending.slice(-3)) {
        info.push(`  pending: ${n.target_role} ← ${n.source_role}: ${(n.summary || "").slice(0, 80)}`);
      }
      if (latestPushAlert) {
        info.push(`  push_alert: ${formatPushAlertInline(latestPushAlert)}`);
      }
    } else {
      info.push("Notifications: all acknowledged");
    }
  } else {
    info.push("Notifications: no file yet");
  }
} catch {}

// 5. Receipts (last activity)
try {
  const commDir = path.join(REPO_ROOT, "..", "gov_runtime", "roles_shared", "WP_COMMUNICATIONS", wpId);
  const receiptsFile = path.join(commDir, "RECEIPTS.jsonl");
  if (fs.existsSync(receiptsFile)) {
    const receipts = parseJsonlFile(receiptsFile);
    const lastReceipt = receipts[receipts.length - 1];
    if (lastReceipt) {
      const lastTime = lastReceipt.timestamp_utc || lastReceipt.timestamp || "";
      const ageMs = Date.now() - new Date(lastTime).getTime();
      const ageMins = Math.round(ageMs / 60000);
      info.push(`Last receipt: ${ageMins}m ago — ${lastReceipt.receipt_kind || "unknown"} from ${lastReceipt.actor_role || "unknown"}`);
      if (ageMins > 10) issues.push(`Last receipt was ${ageMins}m ago — possible stall (default stale_after: 20m)`);
    }
  }
} catch {}

// 6. Runtime-native relay policy
if (relayPolicy && typeof relayPolicy === "object") {
  info.push(
    `Relay policy: ${relayPolicy.failure_class} | ${relayPolicy.policy_state} -> ${relayPolicy.next_strategy} | budget=${relayEscalationPolicyBudgetLabel(relayPolicy)}`,
  );
  info.push(`  policy summary: ${relayPolicy.summary}`);
  if (relayPolicy.policy_state === "AUTO_RETRY_BLOCKED" || relayPolicy.next_strategy === "HUMAN_STOP") {
    issues.push(
      `Relay policy blocks automatic recovery: ${relayPolicy.failure_class} -> ${relayPolicy.next_strategy} (${relayPolicy.reason_code})`,
    );
  }
}

// 7. Token usage and budget
try {
  const { ledger } = readWpTokenUsageLedger(REPO_ROOT, wpId);
  if (ledger && (ledger.summary?.command_count || 0) > 0) {
    const summary = ledger.summary;
    const totals = summary.usage_totals || {};
    const inputK = Math.round((totals.input_tokens || 0) / 1000);
    const outputK = Math.round((totals.output_tokens || 0) / 1000);
    info.push(`Tokens: ${summary.command_count} cmds, ${summary.turn_count} turns | in=${inputK}k out=${outputK}k`);
    const budget = evaluateWpTokenBudget(ledger);
    if (budget.status === "WARN") issues.push(`Token budget WARNING: ${budget.summary}`);
    else if (budget.status === "FAIL") issues.push(`Token budget FAIL: ${budget.summary}`);
    else info.push(`Budget: PASS`);
    const roleTotals = ledger.role_totals || {};
    for (const role of ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]) {
      const rd = roleTotals[role];
      if (rd && (rd.command_count || 0) > 0) {
        const ri = Math.round((rd.usage_totals?.input_tokens || 0) / 1000);
        info.push(`  ${role}: ${rd.command_count} cmds, ${rd.turn_count} turns | in=${ri}k`);
      }
    }
  } else {
    info.push("Tokens: no usage recorded");
  }
} catch {}

// 8. Summary
console.log(`\nWP_LANE_HEALTH: ${wpId}`);
console.log("─".repeat(60));
for (const line of info) console.log(`  ${line}`);
if (issues.length === 0) {
  console.log(`\n  HEALTH: OK (no issues detected)`);
} else {
  console.log(`\n  ISSUES (${issues.length}):`);
  for (const issue of issues) console.log(`  ⚠ ${issue}`);
}
console.log("");
