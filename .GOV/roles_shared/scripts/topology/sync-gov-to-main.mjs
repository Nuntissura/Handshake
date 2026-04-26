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
import { execFileSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  WORKTREE_SPECS,
  currentBranchInRepo,
  formatProtectedWorktreeResolutionDiagnostics,
  gitCheckoutExists,
  headShaInRepo,
  resolveProtectedWorktree,
  runGitInRepo,
  runGitInherit,
} from "./git-topology-lib.mjs";
import { GOV_ROOT_ABS } from "../lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("sync-gov-to-main.mjs", { role: "SHARED" });

const PREFIX = "[SYNC_GOV_TO_MAIN]";

function fail(message, details = []) {
  const detailRows = Array.isArray(details) ? details.filter(Boolean) : [String(details || "")].filter(Boolean);
  failWithMemory("sync-gov-to-main.mjs", [message, ...detailRows].join("\n"), { role: "SHARED", details: detailRows });
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

function normalizeRepoPath(value) {
  return String(value || "").replace(/\\/g, "/").replace(/^\.\//, "");
}

function pathTouchesGov(value) {
  const normalized = normalizeRepoPath(value);
  return normalized === ".GOV" || normalized.startsWith(".GOV/");
}

function pathsFromPorcelainEntry(line) {
  const raw = String(line || "");
  if (raw.length < 4) return [];
  const pathPortion = raw.slice(3).trim();
  if (!pathPortion) return [];
  return pathPortion
    .split(" -> ")
    .map((entry) => normalizeRepoPath(entry))
    .filter(Boolean);
}

export function classifyMainWorktreeGovSyncStatus(statusOutput = "") {
  const entries = String(statusOutput || "")
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter(Boolean)
    .map((line) => {
      const indexStatus = line[0] || " ";
      const worktreeStatus = line[1] || " ";
      const paths = pathsFromPorcelainEntry(line);
      return {
        line,
        indexStatus,
        worktreeStatus,
        paths,
        touchesGov: paths.some((entry) => pathTouchesGov(entry)),
      };
    });

  return {
    entries,
    govEntries: entries.filter((entry) => entry.touchesGov),
    stagedOutsideGovEntries: entries.filter((entry) => !entry.touchesGov && entry.indexStatus !== " " && entry.indexStatus !== "?"),
    unstagedOutsideGovEntries: entries.filter((entry) =>
      !entry.touchesGov
      && (entry.indexStatus === " " || entry.indexStatus === "?")
      && (entry.worktreeStatus !== " " || entry.indexStatus === "?")
    ),
  };
}

function summarizeStatusEntries(entries = [], limit = 5) {
  return entries
    .slice(0, limit)
    .map((entry) => entry.line)
    .join(", ");
}

function readGitStatusPorcelainRaw(repoDir, args = []) {
  return execFileSync(
    "git",
    ["status", "--porcelain=v1", "--untracked-files=all", ...args],
    {
      cwd: repoDir,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    },
  );
}

function runSyncGovToMain() {

// --- Resolve paths ---

const mainSpec = WORKTREE_SPECS.find((s) => s.canonical);
if (!mainSpec) fail("No canonical worktree found in WORKTREE_SPECS");

const mainWorktreeOverrideAbs = parseMainWorktreeOverride(process.argv);
const mainResolution = resolveProtectedWorktree(mainSpec);
const mainWorktreeAbs = mainWorktreeOverrideAbs || mainResolution.absDir;
const mainGovAbs = path.join(mainWorktreeAbs, ".GOV");

const kernelSpec = WORKTREE_SPECS.find((s) => s.role === "GOV_KERNEL");
const kernelResolution = kernelSpec ? resolveProtectedWorktree(kernelSpec) : null;
const kernelWorktreeAbs = kernelResolution?.absDir || null;
const kernelGovAbs = GOV_ROOT_ABS;

// --- Pre-flight checks ---

if (!fs.existsSync(kernelGovAbs)) {
  fail(`Kernel .GOV/ not found at: ${kernelGovAbs}`);
}

if (!fs.existsSync(mainWorktreeAbs) || !gitCheckoutExists(mainWorktreeAbs)) {
  fail(
    `Main worktree not found or not a git checkout: ${mainWorktreeAbs}`,
    mainWorktreeOverrideAbs
      ? [`override_path=${mainWorktreeOverrideAbs}`, ...formatProtectedWorktreeResolutionDiagnostics(mainResolution)]
      : formatProtectedWorktreeResolutionDiagnostics(mainResolution),
  );
}

if (!kernelWorktreeAbs || !gitCheckoutExists(kernelWorktreeAbs)) {
  fail(
    `Kernel worktree not found or not a git checkout: ${kernelWorktreeAbs || "<missing>"}`,
    kernelResolution ? formatProtectedWorktreeResolutionDiagnostics(kernelResolution) : [],
  );
}

const kernelBranch = currentBranchInRepo(kernelWorktreeAbs);
if (kernelBranch !== "gov_kernel") {
  fail(
    `Kernel worktree is on branch '${kernelBranch}', expected 'gov_kernel'`,
    kernelResolution ? formatProtectedWorktreeResolutionDiagnostics({ ...kernelResolution, currentBranch: kernelBranch }) : [],
  );
}

const kernelGovDirty = runGitInRepo(kernelWorktreeAbs, ["status", "--porcelain=v1", "--", ".GOV"]);
if (kernelGovDirty.trim()) {
  fail("Kernel .GOV has uncommitted changes. Commit gov_kernel before syncing to main.");
}

const mainBranch = currentBranchInRepo(mainWorktreeAbs);
if (mainBranch !== "main") {
  fail(
    `Main worktree is on branch '${mainBranch}', expected 'main'`,
    mainWorktreeOverrideAbs
      ? [`override_path=${mainWorktreeOverrideAbs}`, ...formatProtectedWorktreeResolutionDiagnostics({ ...mainResolution, currentBranch: mainBranch })]
      : formatProtectedWorktreeResolutionDiagnostics({ ...mainResolution, currentBranch: mainBranch }),
  );
}

const mainStatus = readGitStatusPorcelainRaw(mainWorktreeAbs);
const mainDirtiness = classifyMainWorktreeGovSyncStatus(mainStatus);

if (mainDirtiness.govEntries.length > 0) {
  fail(`Main .GOV has uncommitted changes. Commit or stash before syncing. ${summarizeStatusEntries(mainDirtiness.govEntries)}`);
}

if (mainDirtiness.stagedOutsideGovEntries.length > 0) {
  fail(`Main worktree has staged non-governance changes. Unstage or commit them before syncing. ${summarizeStatusEntries(mainDirtiness.stagedOutsideGovEntries)}`);
}

if (mainDirtiness.unstagedOutsideGovEntries.length > 0) {
  console.log(
    `${PREFIX} allowing unrelated unstaged main drift outside .GOV/: ${summarizeStatusEntries(mainDirtiness.unstagedOutsideGovEntries)}`
  );
}

console.log(`${PREFIX} kernel .GOV/: ${kernelGovAbs}`);
console.log(`${PREFIX} main .GOV/:   ${mainGovAbs}`);
if (mainWorktreeOverrideAbs) {
  console.log(`${PREFIX} main worktree override: ${mainWorktreeOverrideAbs}`);
}

// --- Copy governance kernel to main using robocopy ---

// Directories to exclude from mirror (stay main-local):
//   operator - operator-private workspace
//   runtime  - machine-written state (matches at any depth)
const excludeDirs = ["operator", "runtime"];

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
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  runSyncGovToMain();
}
