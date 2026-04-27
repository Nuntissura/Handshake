#!/usr/bin/env node
/**
 * Git post-commit hook: auto-fires per-MT mechanical review and judgment review.
 *
 * Detects commits matching "feat: MT-NNN" on feat/WP-* branches. The inline
 * mechanical track runs synchronously in this hook; only a mechanical PASS
 * proceeds to the judgment-track WP_VALIDATOR review request.
 *
 * Installation: copy or symlink to .git/hooks/post-commit in the coder worktree.
 * Or call from an existing post-commit hook:
 *   node .GOV/roles_shared/scripts/hooks/post-commit-mt-review-request.mjs
 *
 * Escape hatch:
 *   --legacy-acp-mechanical
 *
 * The hook is non-blocking: if checks fail, the commit is NOT reverted.
 */

import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const legacyAcpMechanical = process.argv.includes("--legacy-acp-mechanical");

function governanceRuntimeRootForRepo(repoRoot) {
  const direct = String(
    process.env.HANDSHAKE_GOV_RUNTIME_ROOT
      || process.env.HANDSHAKE_GOVERNANCE_RUNTIME_ROOT
      || "",
  ).trim();
  if (direct) return path.resolve(direct);
  const product = String(
    process.env.HANDSHAKE_RUNTIME_ROOT
      || process.env.HANDSHAKE_PRODUCT_RUNTIME_ROOT
      || "",
  ).trim();
  if (product) return path.resolve(product, "repo-governance");
  return path.resolve(repoRoot, "..", "gov_runtime");
}

function appendCompileGateLog({ repoRoot, wpId, entry }) {
  try {
    const wpCommsDir = path.join(governanceRuntimeRootForRepo(repoRoot), "roles_shared", "WP_COMMUNICATIONS", wpId);
    if (fs.existsSync(wpCommsDir)) {
      fs.appendFileSync(path.join(wpCommsDir, "COMPILE_GATE_LOG.jsonl"), `${JSON.stringify(entry)}\n`);
    }
  } catch {
    // Best-effort log; never block the commit.
  }
}

function latestCommitHash(repoRoot) {
  try {
    return execSync("git rev-parse HEAD", { cwd: repoRoot, encoding: "utf8" }).trim();
  } catch {
    return "unknown";
  }
}

let repoRoot;
try {
  repoRoot = execSync("git rev-parse --show-toplevel", { encoding: "utf8" }).trim();
} catch {
  process.exit(0);
}

let branch;
try {
  branch = execSync("git rev-parse --abbrev-ref HEAD", { cwd: repoRoot, encoding: "utf8" }).trim();
} catch {
  process.exit(0);
}

const branchMatch = branch.match(/^feat\/(WP-\S+)$/);
if (!branchMatch) process.exit(0);
const wpId = branchMatch[1];

let commitMsg;
try {
  commitMsg = execSync("git log -1 --format=%s", { cwd: repoRoot, encoding: "utf8" }).trim();
} catch {
  process.exit(0);
}

const mtMatch = commitMsg.match(/^feat:\s+(MT-\d+)\s+(.+)$/i);
if (!mtMatch) process.exit(0);
const mtId = mtMatch[1].toUpperCase().replace(/^MT-(\d{1,2})$/, (_m, n) => `MT-${n.padStart(3, "0")}`);
const mtDesc = mtMatch[2];
const commitHash = latestCommitHash(repoRoot);

const cargoTomlPath = path.join(repoRoot, "src", "backend", "handshake_core", "Cargo.toml");
if (fs.existsSync(cargoTomlPath)) {
  console.log("[POST-COMMIT-HOOK] Compile gate: running cargo check...");
  try {
    execSync("cargo check", {
      encoding: "utf8",
      cwd: repoRoot,
      env: {
        ...process.env,
        CARGO_TARGET_DIR: "../Handshake_Artifacts/handshake-cargo-target",
      },
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 180_000,
    });
    console.log("[POST-COMMIT-HOOK] Compile gate PASSED");
    appendCompileGateLog({
      repoRoot,
      wpId,
      entry: {
        timestamp: new Date().toISOString(),
        mt_id: mtId,
        commit: commitHash,
        gate: "COMPILE_PASSED",
      },
    });
  } catch (cargoErr) {
    const stderr = String(cargoErr?.stderr || cargoErr?.message || "");
    const errorPreview = stderr.slice(0, 500);
    console.log("[POST-COMMIT-HOOK] COMPILE GATE FAILED; review request NOT sent.");
    console.log(errorPreview);
    appendCompileGateLog({
      repoRoot,
      wpId,
      entry: {
        timestamp: new Date().toISOString(),
        mt_id: mtId,
        commit: commitHash,
        gate: "COMPILE_FAILED",
        error_preview: errorPreview,
      },
    });
    process.exit(0);
  }
} else {
  console.log(`[POST-COMMIT-HOOK] No Cargo.toml found at ${cargoTomlPath}; skipping compile gate`);
  appendCompileGateLog({
    repoRoot,
    wpId,
    entry: {
      timestamp: new Date().toISOString(),
      mt_id: mtId,
      commit: commitHash,
      gate: "COMPILE_SKIPPED",
      reason: "NO_CARGO_TOML",
    },
  });
}

try {
  const badPaths = [
    path.join(repoRoot, "target"),
    path.join(repoRoot, "src", "backend", "Handshake_Artifacts"),
    path.join(repoRoot, "src", "backend", "handshake_core", "target"),
  ];
  const found = badPaths.filter((candidate) => fs.existsSync(candidate));
  if (found.length > 0) {
    console.log("[POST-COMMIT-HOOK] ARTIFACT HYGIENE WARNING: found wrongly-placed build artifacts:");
    for (const candidate of found) console.log(`  ${candidate}`);
    console.log("[POST-COMMIT-HOOK] These should be under ../Handshake_Artifacts/, not inside the repo.");
  }
} catch {
  // Best-effort check.
}

const coderKey = `CODER:${wpId}`;
const validatorKey = `WP_VALIDATOR:${wpId}`;
const govRoot = path.join(repoRoot, ".GOV");
const mechanicalTrackScript = path.join(govRoot, "roles", "wp_validator", "scripts", "wp-validator-mechanical-track.mjs");
const reviewExchangeScript = path.join(govRoot, "roles_shared", "scripts", "wp", "wp-review-exchange.mjs");
let mechanicalDetails = null;

console.log(`[POST-COMMIT-HOOK] Detected MT commit: ${mtId} - ${mtDesc}`);
console.log(`[POST-COMMIT-HOOK] WP: ${wpId}, Coder: ${coderKey}, Validator: ${validatorKey}`);

if (legacyAcpMechanical) {
  console.log("[POST-COMMIT-HOOK] --legacy-acp-mechanical set; skipping inline mechanical helper");
} else if (!fs.existsSync(mechanicalTrackScript)) {
  console.log(`[POST-COMMIT-HOOK] Mechanical helper missing at ${mechanicalTrackScript}; review request NOT sent`);
  process.exit(0);
} else {
  console.log(`[POST-COMMIT-HOOK] Running inline mechanical track for ${mtId}...`);
  try {
    const output = execFileSync(process.execPath, [
      mechanicalTrackScript,
      wpId,
      mtId,
      "--range",
      "HEAD~1..HEAD",
      "--json",
    ], {
      encoding: "utf8",
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 30000,
    });
    const parsed = JSON.parse(output);
    mechanicalDetails = parsed.details || null;
    console.log(`[POST-COMMIT-HOOK] ${parsed.content}`);
    if (parsed.details?.verdict !== "PASS") {
      console.log("[POST-COMMIT-HOOK] Mechanical track did not pass; judgment review request NOT sent");
      process.exit(0);
    }
  } catch (error) {
    const stdout = String(error?.stdout || "").trim();
    try {
      const parsed = JSON.parse(stdout);
      console.log(`[POST-COMMIT-HOOK] ${parsed.content || "Mechanical track failed"}`);
      if (parsed.details?.verdict === "FAIL") {
        console.log("[POST-COMMIT-HOOK] Mechanical track failed; judgment review request NOT sent");
        process.exit(0);
      }
    } catch {
      // Fall through to generic failure handling.
    }
    const raw = String(error?.stderr || stdout || error?.message || error || "").trim();
    console.log(`[POST-COMMIT-HOOK] Mechanical helper failed; review request NOT sent: ${raw.slice(0, 300)}`);
    process.exit(0);
  }
}

if (!fs.existsSync(reviewExchangeScript)) {
  console.log(`[POST-COMMIT-HOOK] wp-review-exchange.mjs not found at ${reviewExchangeScript}; skipping auto-relay`);
  process.exit(0);
}

console.log("[POST-COMMIT-HOOK] Firing judgment-track wp-review-request for auto-relay...");

const microtaskJson = mechanicalDetails
  ? JSON.stringify({
    scope_ref: mtId,
    file_targets: mechanicalDetails.file_list_match_result?.declared_code_surfaces || [],
    proof_commands: mechanicalDetails.expected_tests || [],
    expected_receipt_kind: "REVIEW_RESPONSE",
    review_mode: "OVERLAP",
    phase_gate: "MICROTASK",
  })
  : "";

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
    "",
    "",
    mtId,
    "",
    microtaskJson,
  ], {
    encoding: "utf8",
    cwd: repoRoot,
    stdio: ["ignore", "pipe", "pipe"],
    timeout: 15000,
  });
  const lines = output.trim().split(/\r?\n/).filter(Boolean).slice(-5);
  for (const line of lines) console.log(`[POST-COMMIT-HOOK] ${line}`);
  console.log("[POST-COMMIT-HOOK] Auto-relay fired successfully");
} catch (error) {
  const msg = error?.stderr || error?.stdout || error?.message || String(error);
  console.log(`[POST-COMMIT-HOOK] Auto-relay failed (non-fatal): ${String(msg).slice(0, 200)}`);
}
