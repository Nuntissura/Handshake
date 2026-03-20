#!/usr/bin/env node

/**
 * sync-gov-to-main.mjs
 *
 * Copies the governance kernel .GOV/ directory into the handshake_main worktree,
 * writes a sync marker, and auto-commits on main.
 *
 * RESPONSIBILITY [CX-212D]: This script is owned by the Integration
 * Validator by default before pushing to origin/main. The Orchestrator MAY
 * also call it when the Operator explicitly instructs that mechanical
 * governance/main sync execution.
 *
 * Usage: node .GOV/roles_shared/scripts/topology/sync-gov-to-main.mjs
 *        node .GOV/roles_shared/scripts/topology/sync-gov-to-main.mjs --main-worktree <abs-or-rel-path>
 *        just sync-gov-to-main
 */

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  WORKTREE_SPECS,
  absFromRepo,
  currentBranchInRepo,
  dirtyInRepo,
  gitCheckoutExists,
  headShaInRepo,
  runGitInRepo,
  runGitInherit,
} from "./git-topology-lib.mjs";
import { GOV_ROOT_ABS } from "../lib/runtime-paths.mjs";

const PREFIX = "[SYNC_GOV_TO_MAIN]";

function fail(message) {
  console.error(`${PREFIX} FAIL: ${message}`);
  process.exit(1);
}

function parseMainWorktreeOverride(argv) {
  for (let i = 2; i < argv.length; i += 1) {
    if (argv[i] === "--main-worktree") {
      const value = argv[i + 1];
      if (!value || value.startsWith("--")) {
        fail("Expected a path after --main-worktree");
      }
      return path.resolve(value);
    }
  }

  const envValue = process.env.HANDSHAKE_MAIN_WORKTREE_OVERRIDE?.trim();
  return envValue ? path.resolve(envValue) : null;
}

// --- Resolve paths ---

const mainSpec = WORKTREE_SPECS.find((s) => s.canonical);
if (!mainSpec) fail("No canonical worktree found in WORKTREE_SPECS");

const mainWorktreeOverrideAbs = parseMainWorktreeOverride(process.argv);
const mainWorktreeAbs = mainWorktreeOverrideAbs || absFromRepo(mainSpec.rel_path);
const mainGovAbs = path.join(mainWorktreeAbs, ".GOV");

const kernelSpec = WORKTREE_SPECS.find((s) => s.role === "GOV_KERNEL");
const kernelWorktreeAbs = kernelSpec ? absFromRepo(kernelSpec.rel_path) : null;
const kernelGovAbs = GOV_ROOT_ABS;

// --- Pre-flight checks ---

if (!fs.existsSync(kernelGovAbs)) {
  fail(`Kernel .GOV/ not found at: ${kernelGovAbs}`);
}

if (!fs.existsSync(mainWorktreeAbs) || !gitCheckoutExists(mainWorktreeAbs)) {
  fail(`Main worktree not found or not a git checkout: ${mainWorktreeAbs}`);
}

const mainBranch = currentBranchInRepo(mainWorktreeAbs);
if (mainBranch !== "main") {
  fail(`Main worktree is on branch '${mainBranch}', expected 'main'`);
}

if (dirtyInRepo(mainWorktreeAbs)) {
  fail("Main worktree has uncommitted changes. Commit or stash before syncing.");
}

console.log(`${PREFIX} kernel .GOV/: ${kernelGovAbs}`);
console.log(`${PREFIX} main .GOV/:   ${mainGovAbs}`);
if (mainWorktreeOverrideAbs) {
  console.log(`${PREFIX} main worktree override: ${mainWorktreeOverrideAbs}`);
}

// --- Copy governance kernel to main using robocopy ---

// Directories to exclude from mirror (stay main-local):
//   Audits   - audit outputs belong in main, not the kernel
//   operator - operator-private workspace
//   runtime  - machine-written state (matches at any depth)
const excludeDirs = ["Audits", "operator", "runtime"];

const robocopyArgs = [
  kernelGovAbs,
  mainGovAbs,
  "/MIR",
  "/COPY:DAT",
  "/R:1",
  "/W:1",
  "/XJ",
  "/NFL",
  "/NDL",
  "/NJH",
  "/NJS",
  "/NP",
  "/XD",
  ...excludeDirs,
];

console.log(`${PREFIX} mirroring kernel .GOV/ -> main .GOV/ (excluding: ${excludeDirs.join(", ")})`);

const robocopyResult = spawnSync("robocopy", robocopyArgs, { stdio: "inherit" });
if (typeof robocopyResult.status === "number" && robocopyResult.status >= 8) {
  fail(`robocopy failed with exit code ${robocopyResult.status}`);
}

// --- Write sync marker ---

const kernelHeadSha = kernelWorktreeAbs
  ? headShaInRepo(kernelWorktreeAbs)
  : "unknown";

const syncMarker = {
  schema_id: "hsk.gov_kernel_sync@0.1",
  source_commit: kernelHeadSha,
  source_branch: kernelSpec ? kernelSpec.local_branch : "unknown",
  source_worktree: kernelSpec ? kernelSpec.rel_path : "unknown",
  sync_timestamp: new Date().toISOString(),
  synced_by: "sync-gov-to-main.mjs",
};

const markerPath = path.join(mainGovAbs, "GOV_KERNEL_SYNC.json");
fs.writeFileSync(markerPath, JSON.stringify(syncMarker, null, 2) + "\n", "utf8");
console.log(`${PREFIX} wrote sync marker: ${markerPath}`);

// --- Stage and commit on main ---

runGitInRepo(mainWorktreeAbs, ["add", ".GOV/"]);

const statusOutput = runGitInRepo(mainWorktreeAbs, ["status", "--porcelain=v1"]);
if (!statusOutput.trim()) {
  console.log(`${PREFIX} no changes detected - main .GOV/ already matches kernel`);
  process.exit(0);
}

const shortSha = kernelHeadSha.slice(0, 7);
const commitMessage = `gov: sync governance kernel ${shortSha}`;

runGitInherit(mainWorktreeAbs, ["commit", "-m", commitMessage]);
console.log(`${PREFIX} committed on main: ${commitMessage}`);
console.log(`${PREFIX} done - push main when ready: git -C ${mainWorktreeAbs} push origin main`);
