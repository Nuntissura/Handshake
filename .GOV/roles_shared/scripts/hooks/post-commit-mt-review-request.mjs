#!/usr/bin/env node
/**
 * Git post-commit hook: auto-fires wp-review-request when the coder commits an MT.
 *
 * Detects commits matching "feat: MT-NNN" pattern and automatically creates a
 * governed review request notification, triggering the auto-relay to the validator.
 *
 * Installation: copy or symlink to .git/hooks/post-commit in the coder worktree.
 * Or call from an existing post-commit hook:
 *   node .GOV/roles_shared/scripts/hooks/post-commit-mt-review-request.mjs
 *
 * The hook is non-blocking: if the review request fails, the commit is NOT reverted.
 */

import { execFileSync, execSync } from "node:child_process";
import path from "node:path";
import fs from "node:fs";

// Determine repo root and WP-ID from branch name
let repoRoot;
try {
  repoRoot = execSync("git rev-parse --show-toplevel", { encoding: "utf8" }).trim();
} catch {
  process.exit(0); // Not in a git repo
}

let branch;
try {
  branch = execSync("git rev-parse --abbrev-ref HEAD", { encoding: "utf8" }).trim();
} catch {
  process.exit(0);
}

// Extract WP-ID from branch name (feat/WP-1-...)
const branchMatch = branch.match(/^feat\/(WP-\S+)$/);
if (!branchMatch) {
  process.exit(0); // Not a WP feature branch
}
const wpId = branchMatch[1];

// Get the latest commit message
let commitMsg;
try {
  commitMsg = execSync("git log -1 --format=%s", { encoding: "utf8" }).trim();
} catch {
  process.exit(0);
}

// Check if it's an MT commit
const mtMatch = commitMsg.match(/^feat:\s+(MT-\d+)\s+(.+)$/i);
if (!mtMatch) {
  process.exit(0); // Not an MT commit
}
const mtId = mtMatch[1];
const mtDesc = mtMatch[2];

// Build session keys
const coderKey = `CODER:${wpId}`;
const validatorKey = `WP_VALIDATOR:${wpId}`;

// Find the wp-review-request script
const govRoot = path.join(repoRoot, ".GOV");
const reviewExchangeScript = path.join(govRoot, "roles_shared", "scripts", "wp", "wp-review-exchange.mjs");

if (!fs.existsSync(reviewExchangeScript)) {
  console.log(`[POST-COMMIT-HOOK] wp-review-exchange.mjs not found at ${reviewExchangeScript}; skipping auto-relay`);
  process.exit(0);
}

console.log(`[POST-COMMIT-HOOK] Detected MT commit: ${mtId} — ${mtDesc}`);
console.log(`[POST-COMMIT-HOOK] WP: ${wpId}, Coder: ${coderKey}, Validator: ${validatorKey}`);
console.log(`[POST-COMMIT-HOOK] Firing wp-review-request for auto-relay...`);

try {
  const output = execFileSync(process.execPath, [
    reviewExchangeScript,
    "REVIEW_REQUEST",
    wpId,
    "CODER",
    coderKey,
    "WP_VALIDATOR",
    validatorKey,
    `${mtId} complete: ${mtDesc}`,
    "", // correlation_id
    "", // spec_anchor
    "", // packet_row_ref
    "", // ack_for
    "", // microtask_json (empty — relaxed for REVIEW_REQUEST per RGF-96 fix)
  ], {
    encoding: "utf8",
    cwd: repoRoot,
    stdio: ["ignore", "pipe", "pipe"],
    timeout: 15000, // 15s max — don't block the commit
  });
  const lines = output.trim().split(/\r?\n/).filter(Boolean).slice(-5);
  for (const line of lines) console.log(`[POST-COMMIT-HOOK] ${line}`);
  console.log(`[POST-COMMIT-HOOK] Auto-relay fired successfully`);
} catch (error) {
  // Non-fatal: if auto-relay fails, the commit still succeeds.
  // The orchestrator can manually dispatch the review request.
  const msg = error?.stderr || error?.stdout || error?.message || String(error);
  console.log(`[POST-COMMIT-HOOK] Auto-relay failed (non-fatal): ${String(msg).slice(0, 200)}`);
}
