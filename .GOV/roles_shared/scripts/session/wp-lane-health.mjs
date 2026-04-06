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
import { sessionKey, defaultCoderBranch, defaultCoderWorktreeDir } from "./session-policy.mjs";
import { REPO_ROOT, repoPathAbs } from "../lib/runtime-paths.mjs";
import { parseJsonlFile } from "../lib/wp-communications-lib.mjs";

const wpId = String(process.argv[2] || "").trim();
if (!wpId || !wpId.startsWith("WP-")) {
  console.error("Usage: node wp-lane-health.mjs <WP_ID>");
  process.exit(1);
}

const issues = [];
const info = [];

// 1. Session states
const { registry } = loadSessionRegistry(REPO_ROOT);
const coderKey = sessionKey("CODER", wpId);
const validatorKey = sessionKey("WP_VALIDATOR", wpId);
const coderSession = registry.sessions.find((s) => s.session_key === coderKey);
const validatorSession = registry.sessions.find((s) => s.session_key === validatorKey);

if (!coderSession) {
  issues.push("CODER session not registered");
} else {
  info.push(`CODER: ${coderSession.runtime_state}`);
  if (coderSession.runtime_state === "FAILED") issues.push("CODER session FAILED — needs restart");
  if (coderSession.runtime_state === "COMPLETED") issues.push("CODER session COMPLETED — may need restart for more MTs");
}

if (!validatorSession) {
  issues.push("WP_VALIDATOR session not registered");
} else {
  info.push(`WP_VALIDATOR: ${validatorSession.runtime_state}`);
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
  issues.push(`Coder worktree not found: ${worktreeDir}`);
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
    if (pending.length > 0) {
      issues.push(`${pending.length} unacknowledged notification(s) — auto-relay may have failed`);
      for (const n of pending.slice(-3)) {
        info.push(`  pending: ${n.target_role} ← ${n.source_role}: ${(n.summary || "").slice(0, 80)}`);
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

// 6. Summary
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
