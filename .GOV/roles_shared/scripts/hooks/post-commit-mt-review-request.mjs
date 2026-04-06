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

// ── RGF-98: Per-MT Compile Gate ──────────────────────────────────────
// Run cargo check before dispatching the review request.
// If it fails, log the failure and skip the review — but never block the commit.
const cargoTomlPath = path.join(repoRoot, "src", "backend", "handshake_core", "Cargo.toml");
if (fs.existsSync(cargoTomlPath)) {
  console.log(`[POST-COMMIT-HOOK] Compile gate: running cargo check...`);
  let commitHash;
  try {
    commitHash = execSync("git rev-parse HEAD", { encoding: "utf8" }).trim();
  } catch {
    commitHash = "unknown";
  }

  try {
    execSync("cargo check", {
      encoding: "utf8",
      cwd: repoRoot,
      env: {
        ...process.env,
        CARGO_TARGET_DIR: "../Handshake Artifacts/handshake-cargo-target",
      },
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 180_000, // 3 minutes
    });
    console.log(`[POST-COMMIT-HOOK] Compile gate PASSED`);
  } catch (cargoErr) {
    const stderr = String(cargoErr?.stderr || cargoErr?.message || "");
    const errorPreview = stderr.slice(0, 500);
    console.log(`[POST-COMMIT-HOOK] COMPILE GATE FAILED — cargo check returned errors. Review request NOT sent.`);
    console.log(errorPreview);

    // Write failure entry to COMPILE_GATE_LOG.jsonl in the WP communications dir
    try {
      const govRuntimeRoot = (() => {
        const direct = String(process.env.HANDSHAKE_GOVERNANCE_RUNTIME_ROOT || "").trim();
        if (direct) return path.resolve(direct);
        const product = String(process.env.HANDSHAKE_PRODUCT_RUNTIME_ROOT || "").trim();
        if (product) return path.resolve(product, "repo-governance");
        return path.resolve(repoRoot, "..", "gov_runtime");
      })();
      const wpCommsDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
      if (fs.existsSync(wpCommsDir)) {
        const logEntry = JSON.stringify({
          timestamp: new Date().toISOString(),
          mt_id: mtId,
          commit: commitHash,
          gate: "COMPILE_FAILED",
          error_preview: errorPreview,
        });
        fs.appendFileSync(path.join(wpCommsDir, "COMPILE_GATE_LOG.jsonl"), logEntry + "\n");
      }
    } catch {
      // Best-effort log — do not block the commit
    }

    process.exit(0);
  }
} else {
  console.log(`[POST-COMMIT-HOOK] No Cargo.toml found at ${cargoTomlPath} — skipping compile gate`);
}
// ── End RGF-98 ───────────────────────────────────────────────────────

// ── RGF-106: Per-MT Completion Hooks (Artifact Hygiene Gate) ─────────
// Check for wrongly-placed build artifacts inside the codebase.
try {
  const badPaths = [
    path.join(repoRoot, "target"),
    path.join(repoRoot, "src", "backend", "Handshake Artifacts"),
    path.join(repoRoot, "src", "backend", "handshake_core", "target"),
  ];
  const found = badPaths.filter((p) => fs.existsSync(p));
  if (found.length > 0) {
    console.log(`[POST-COMMIT-HOOK] ARTIFACT HYGIENE WARNING: found wrongly-placed build artifacts:`);
    for (const p of found) console.log(`  ${p}`);
    console.log(`[POST-COMMIT-HOOK] These should be under ../Handshake Artifacts/, not inside the repo.`);
    // Warning only — does not block the review request
  }
} catch {
  // Best-effort check
}
// ── End RGF-106 ──────────────────────────────────────────────────────

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
