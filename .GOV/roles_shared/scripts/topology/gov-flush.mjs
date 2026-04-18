#!/usr/bin/env node

/**
 * gov-flush.mjs
 *
 * Deterministic governance flush pipeline:
 *   1. Commit dirty .GOV/ files in wt-gov-kernel + push to origin/gov_kernel
 *   2. Sync gov to main (just sync-gov-to-main)
 *   3. Push main to origin/main
 *   4. Reseed wt-ilja from main (auto-approved)
 *   5. Push user_ilja to origin/user_ilja
 *   6. Memory hygiene (mechanical pre-pass — extraction, decay, consolidation, recall audit)
 *   7. Artifact cleanup (dry-run then actual; no force delete)
 *   8. Backup snapshot to NAS (only if cleanup succeeded; 10 min timeout for slow NAS)
 *
 * Steps 1-5 are atomic in sequence: failure stops the pipeline.
 * Step 6 failure is non-blocking (warns, continues to backup current DB state).
 * Step 7 failure is reported but does NOT undo steps 1-5.
 * Step 8 runs only if step 7 succeeded. Memory DB is cleaned by step 6 before backup.
 *
 * Usage: just gov-flush
 */

import fs from "node:fs";
import path from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import {
  REPO_ROOT,
  WORKSPACE_ROOT,
  WORKTREE_SPECS,
  absFromRepo,
  currentBranchInRepo,
  dirtyInRepo,
  headShaInRepo,
  runGit,
  runGitInRepo,
  runGitInherit,
} from "./git-topology-lib.mjs";
import { ensureGovKernelTracksGov } from "./reseed-permanent-worktree-from-main.mjs";
import { GOV_ROOT_ABS } from "../lib/runtime-paths.mjs";
import { registerFailCaptureHook, captureFailure } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("gov-flush.mjs", { role: "SHARED" });

const PREFIX = "[GOV_FLUSH]";
const report = { steps: [], warnings: [], errors: [] };
const MEMORY_HYGIENE_TIMEOUT_MS = 10 * 60 * 1000;
const BACKUP_SNAPSHOT_TIMEOUT_MS = 20 * 60 * 1000;

function log(msg) { console.log(`${PREFIX} ${msg}`); }
function warn(msg) { report.warnings.push(msg); console.warn(`${PREFIX} WARN: ${msg}`); }
function fail(msg) {
  captureFailure("gov-flush.mjs", msg, { role: "SHARED" });
  report.errors.push(msg);
  console.error(`${PREFIX} FAIL: ${msg}`);
  printReport();
  process.exit(1);
}

function step(name, detail = "") {
  const entry = { name, detail, status: "OK" };
  report.steps.push(entry);
  log(`── ${name}${detail ? ` — ${detail}` : ""}`);
  return entry;
}

function printReport() {
  console.log(`\n${PREFIX} ═══ FLUSH REPORT ═══`);
  for (const s of report.steps) {
    const marker = s.status === "OK" ? "PASS" : s.status === "SKIP" ? "SKIP" : "FAIL";
    console.log(`  [${marker}] ${s.name}${s.detail ? ` — ${s.detail}` : ""}`);
  }
  if (report.warnings.length > 0) {
    console.log(`\n${PREFIX} WARNINGS:`);
    for (const w of report.warnings) console.log(`  ! ${w}`);
  }
  if (report.errors.length > 0) {
    console.log(`\n${PREFIX} ERRORS:`);
    for (const e of report.errors) console.log(`  ✗ ${e}`);
  }
  console.log();
}

function runJust(recipe, args = []) {
  const result = spawnSync("just", [recipe, ...args], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    shell: true,
  });
  return { exitCode: result.status, stdout: result.stdout || "", stderr: result.stderr || "" };
}

function runTimedSpawnSync(command, args, options = {}) {
  const nextOptions = { ...options };
  if (nextOptions.timeout === 120000) nextOptions.timeout = MEMORY_HYGIENE_TIMEOUT_MS;
  if (nextOptions.timeout === 600000) nextOptions.timeout = BACKUP_SNAPSHOT_TIMEOUT_MS;
  return spawnSync(command, args, nextOptions);
}

// ─── Resolve worktree paths ─────────────────────────────────────
const GOV_KERNEL_SPEC = WORKTREE_SPECS.find(s => s.id === "wt-gov-kernel");
const MAIN_SPEC = WORKTREE_SPECS.find(s => s.id === "handshake_main");
const ILJA_SPEC = WORKTREE_SPECS.find(s => s.id === "wt-ilja");
const govKernelAbs = absFromRepo(GOV_KERNEL_SPEC.rel_path);
const mainAbs = absFromRepo(MAIN_SPEC.rel_path);
const iljaAbs = absFromRepo(ILJA_SPEC.rel_path);

// ─── Step 1: Commit all dirty governance files + push gov_kernel ─
{
  const s = step("commit-gov-kernel", "commit all dirty files in wt-gov-kernel + push to origin/gov_kernel");
  const branch = currentBranchInRepo(govKernelAbs);
  if (branch !== "gov_kernel") fail(`wt-gov-kernel is on branch '${branch}', expected 'gov_kernel'`);

  ensureGovKernelTracksGov(govKernelAbs);

  // Stage all governance changes in wt-gov-kernel:
  //   1. normalize local .GOV suppression so wt-gov-kernel tracks new governance files
  //   2. git add .GOV/ — stages new and modified governance files normally
  //   2. git add -u — catches tracked modifications (justfile, etc.)
  // wt-gov-kernel is governance-only, so all dirty files are governance files.
  try { runGitInRepo(govKernelAbs, ["add", ".GOV/"]); } catch { /* no new .GOV/ files */ }
  try { runGitInRepo(govKernelAbs, ["add", "-u"]); } catch { /* no tracked modifications */ }

  // Check what's actually staged
  const staged = runGitInRepo(govKernelAbs, ["diff", "--cached", "--name-only"])
    .split("\n").filter(l => l.trim());

  if (staged.length === 0) {
    s.detail = "no dirty files — skip commit";
    s.status = "SKIP";
    log("  no dirty files, skipping commit");
  } else {
    log(`  staged files (${staged.length}):`);
    for (const f of staged) log(`    ${f}`);

    // Auto-generate commit message from file list
    const fileBasenames = staged.map(f => path.basename(f));
    const uniqueNames = [...new Set(fileBasenames)];
    const shortList = uniqueNames.length <= 5
      ? uniqueNames.join(", ")
      : `${uniqueNames.slice(0, 4).join(", ")} + ${uniqueNames.length - 4} more`;
    const commitMsg = `gov: flush — ${shortList}`;

    runGitInRepo(govKernelAbs, ["commit", "-m", commitMsg]);
    log(`  committed: ${commitMsg}`);
  }

  // Push gov_kernel to origin regardless (in case there were prior unpushed commits)
  log("  pushing gov_kernel to origin...");
  try {
    runGitInRepo(govKernelAbs, ["push", "origin", "gov_kernel"]);
    log("  pushed to origin/gov_kernel");
  } catch (e) {
    fail(`push to origin/gov_kernel failed: ${e.message || e}`);
  }
}

// ─── Step 2: Sync gov to main ───────────────────────────────────
{
  const s = step("sync-gov-to-main", "mirror .GOV/ into handshake_main + auto-commit on main");
  const result = runJust("sync-gov-to-main");
  if (result.exitCode !== 0) {
    s.status = "FAIL";
    fail(`sync-gov-to-main failed (exit ${result.exitCode}): ${result.stderr.trim() || result.stdout.trim()}`);
  }
  log("  sync-gov-to-main completed");
  if (result.stdout.trim()) {
    for (const line of result.stdout.trim().split("\n")) log(`  ${line}`);
  }
}

// ─── Step 3: Push main to origin/main ───────────────────────────
{
  const s = step("push-main", "push handshake_main to origin/main");
  try {
    runGitInRepo(mainAbs, ["push", "origin", "main"]);
    log("  pushed to origin/main");
  } catch (e) {
    s.status = "FAIL";
    fail(`push to origin/main failed: ${e.message || e}`);
  }
}

// ─── Step 4: Reseed wt-ilja from main ───────────────────────────
{
  const s = step("reseed-wt-ilja", "reseed wt-ilja from local main (auto-approved)");
  const result = runJust("reseed-permanent-worktree-from-main", ["wt-ilja", "approved", `gov-flush-${new Date().toISOString().replace(/[:.]/g, "").slice(0, 15)}Z`]);
  if (result.exitCode !== 0) {
    s.status = "FAIL";
    fail(`reseed wt-ilja failed (exit ${result.exitCode}): ${result.stderr.trim() || result.stdout.trim()}`);
  }
  log("  wt-ilja reseeded from main");
  if (result.stdout.trim()) {
    for (const line of result.stdout.trim().split("\n")) log(`  ${line}`);
  }
}

// ─── Step 5: Push user_ilja to origin/user_ilja ─────────────────
{
  const s = step("push-user-ilja", "push wt-ilja to origin/user_ilja");
  try {
    runGitInRepo(iljaAbs, ["push", "origin", "user_ilja"]);
    log("  pushed to origin/user_ilja");
  } catch (e) {
    s.status = "FAIL";
    fail(`push to origin/user_ilja failed: ${e.message || e}`);
  }
}

// ─── Step 6: Memory hygiene (mechanical pre-pass) ──────────────
{
  const s = step("memory-hygiene", "mechanical memory maintenance — extraction, decay, consolidation, recall audit");
  const result = runTimedSpawnSync("just", ["launch-memory-manager", "--force"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    shell: true,
    timeout: 120000, // 2 min — embedding refresh can be slow
  });
  if (result.status !== 0) {
    s.status = "FAIL";
    s.detail = "memory hygiene failed — continuing with backup of current DB state";
    warn(`launch-memory-manager failed (exit ${result.status}): ${(result.stderr || "").trim() || (result.stdout || "").trim()}`);
  } else {
    log("  memory hygiene completed");
    if ((result.stdout || "").trim()) {
      for (const line of result.stdout.trim().split("\n")) log(`  ${line}`);
    }
  }
}

// ─── Step 7: Artifact cleanup ───────────────────────────────────
let artifactCleanupOk = false;
{
  const s = step("artifact-cleanup", "dry-run then actual cleanup of Handshake_Artifacts");

  // Dry run first
  log("  running artifact-cleanup --dry-run...");
  const dryResult = runJust("artifact-cleanup", ["--dry-run"]);
  if (dryResult.exitCode !== 0) {
    s.status = "FAIL";
    s.detail = "dry-run failed — stopping cleanup, no deletion attempted";
    warn(`artifact-cleanup dry-run failed (exit ${dryResult.exitCode})`);
    if (dryResult.stderr.trim()) warn(`  stderr: ${dryResult.stderr.trim()}`);
    if (dryResult.stdout.trim()) {
      for (const line of dryResult.stdout.trim().split("\n")) log(`  ${line}`);
    }
  } else {
    log("  dry-run passed, proceeding with actual cleanup...");
    if (dryResult.stdout.trim()) {
      for (const line of dryResult.stdout.trim().split("\n")) log(`  ${line}`);
    }

    // Actual cleanup
    const cleanResult = runJust("artifact-cleanup");
    if (cleanResult.exitCode !== 0) {
      s.status = "FAIL";
      s.detail = "cleanup failed — stopped, no force delete, operator must inspect";
      warn(`artifact-cleanup failed (exit ${cleanResult.exitCode})`);
      if (cleanResult.stderr.trim()) warn(`  stderr: ${cleanResult.stderr.trim()}`);
      if (cleanResult.stdout.trim()) {
        for (const line of cleanResult.stdout.trim().split("\n")) log(`  ${line}`);
      }
    } else {
      artifactCleanupOk = true;
      log("  artifact cleanup completed");
      if (cleanResult.stdout.trim()) {
        for (const line of cleanResult.stdout.trim().split("\n")) log(`  ${line}`);
      }
    }
  }
}

// ─── Step 8: Backup to NAS ──────────────────────────────────────
{
  const s = step("backup-snapshot-nas", "immutable snapshot to local + NAS (memory DB cleaned by step 6)");
  if (!artifactCleanupOk) {
    s.status = "SKIP";
    s.detail = "skipped — artifact cleanup did not succeed";
    warn("NAS backup skipped because artifact cleanup failed. Governance is safe on GitHub but local artifacts need operator inspection before NAS backup.");
  } else {
    const label = `gov-flush-${new Date().toISOString().replace(/[:.]/g, "").slice(0, 15)}Z`;
    log("  creating backup snapshot (NAS may be slow — 10 min timeout)...");
    const result = runTimedSpawnSync("just", ["backup-snapshot", label], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      shell: true,
      timeout: 600000, // 10 minutes — NAS can be slow
    });
    if (result.status !== 0) {
      s.status = "FAIL";
      warn(`backup-snapshot failed (exit ${result.status}): ${(result.stderr || "").trim() || (result.stdout || "").trim()}`);
    } else {
      log("  backup snapshot completed");
      if ((result.stdout || "").trim()) {
        for (const line of result.stdout.trim().split("\n")) log(`  ${line}`);
      }
    }
  }
}

// ─── Final report ───────────────────────────────────────────────
printReport();

const hasErrors = report.errors.length > 0;
const hasWarnings = report.warnings.length > 0;
if (hasErrors) {
  log("gov-flush FAIL");
  process.exit(1);
} else if (hasWarnings) {
  log("gov-flush WARN (governance secured on GitHub, but artifact/backup issues need operator attention)");
} else {
  log("gov-flush ok");
}
