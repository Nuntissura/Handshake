#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  REPO_ROOT,
  WORKTREE_SPECS,
  absFromRepo,
  currentBranchInRepo,
  dirtyInRepo,
  dirtyOutsideGovInRepo,
  gitCheckoutExists,
  localBranchExists,
  refExists,
  runGitInRepo,
  runGitInherit,
} from "./git-topology-lib.mjs";
import { detachExternalGovLink } from "./delete-local-worktree.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("reseed-permanent-worktree-from-main.mjs", { role: "SHARED" });

const RESEEDABLE_WORKTREE_ROLES = new Set(["OPERATOR"]);
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const SHARED_GOV_EXCLUDE_MARKER = "# HANDSHAKE_SHARED_GOV_JUNCTION";
const SHARED_GOV_EXCLUDE_RULE = ".GOV/";
const GOV_KERNEL_SPEC = WORKTREE_SPECS.find((entry) => entry.id === "wt-gov-kernel");
const GOV_KERNEL_WORKTREE_ABS = GOV_KERNEL_SPEC ? absFromRepo(GOV_KERNEL_SPEC.rel_path) : "";
const CLEAN_SETTLE_ATTEMPTS = 30;
const CLEAN_SETTLE_DELAY_MS = 2000;

function normalizeComparablePath(value) {
  const normalized = path.resolve(String(value || "")).replace(/\\/g, "/").replace(/\/+$/, "");
  return process.platform === "win32" ? normalized.toLowerCase() : normalized;
}

function sleepSync(ms) {
  const delayMs = Math.max(0, Number(ms) || 0);
  if (!delayMs) return;
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, delayMs);
}

function removeDirectoryLinkOnly(linkPath) {
  if (process.platform === "win32") {
    // Use fs.rmdirSync for junctions — it calls Win32 RemoveDirectory which
    // correctly removes the reparse point without following the junction.
    // Previous cmd /c rmdir approach silently failed on paths with spaces.
    fs.rmdirSync(linkPath);
    return;
  }
  fs.unlinkSync(linkPath);
}

function fail(message, details = []) {
  failWithMemory("reseed-permanent-worktree-from-main.mjs", message, { role: "SHARED", details });
}

function usage() {
  fail(
    "Usage: node .GOV/roles_shared/scripts/topology/reseed-permanent-worktree-from-main.mjs <WORKTREE_ID> --approve \"approved|proceed\" [--label <snapshot-label>]",
    [
      "This helper safety-pushes the matching backup branch, creates an immutable snapshot, resets the local role/user branch to local main, and repairs the .GOV junction.",
      "Supported permanent worktrees: wt-ilja",
    ],
  );
}

function normalizeApproval(value) {
  return String(value || "").trim().toLowerCase();
}

function parseArgs() {
  const worktreeId = String(process.argv[2] || "").trim();
  if (!worktreeId) usage();

  let approval = "";
  let label = "";
  const args = process.argv.slice(3);
  for (let index = 0; index < args.length; index += 1) {
    const token = String(args[index] || "").trim();
    if (!token) continue;
    const next = () => {
      const value = String(args[index + 1] || "").trim();
      if (!value) usage();
      index += 1;
      return value;
    };

    if (token === "--approve") {
      approval = next();
      continue;
    }
    if (token === "--label") {
      label = next();
      continue;
    }
    fail("Unknown argument", [`arg=${token}`]);
  }

  return { worktreeId, approval, label };
}

function requireApproval(worktreeId, approval) {
  const normalized = normalizeApproval(approval);
  if (normalized === "approved" || normalized === "proceed") return;
  fail("Missing valid approval acknowledgement", [
    "accepted approvals: approved | proceed",
    `worktree_id=${worktreeId}`,
  ]);
}

function createSafetySnapshot(worktreeId, label = "") {
  const snapshotLabel = String(label || "").trim() || `pre-reseed-${worktreeId}-from-main`;
  execFileSync(
    process.execPath,
    [path.join(SCRIPT_DIR, "backup-snapshot.mjs"), "--label", snapshotLabel],
    { cwd: REPO_ROOT, stdio: "inherit" },
  );
}

function ensureGovJunction(absDir) {
  const govDir = path.join(absDir, ".GOV");
  const govKernelAbs = path.resolve(absDir, "..", "wt-gov-kernel", ".GOV");

  if (!fs.existsSync(govKernelAbs)) {
    fail("Governance kernel .GOV path not found", [`expected=${govKernelAbs}`]);
  }

  if (fs.existsSync(govDir)) {
    const stat = fs.lstatSync(govDir);
    if (stat.isSymbolicLink()) {
      try {
        const actualTarget = path.resolve(fs.realpathSync(govDir));
        const expectedTarget = path.resolve(fs.realpathSync(govKernelAbs));
        if (normalizeComparablePath(actualTarget) === normalizeComparablePath(expectedTarget)) {
          console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] .GOV already linked in ${absDir}`);
          return;
        }
      } catch {
        // Fall through to replace an unreadable/broken link below.
      }
      console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] replacing incorrect .GOV junction in ${absDir}`);
      removeDirectoryLinkOnly(govDir);
    } else {
      console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] replacing inherited .GOV with junction in ${absDir}`);
      fs.rmSync(govDir, { recursive: true, force: true });
    }
  }

  // Use Node.js native junction creation — works on all platforms and avoids
  // cmd.exe quoting issues with paths containing spaces.
  fs.symlinkSync(govKernelAbs, govDir, "junction");
  console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] .GOV junction created -> ${govKernelAbs}`);
}

function gitPathInRepo(repoDir, gitPath) {
  const resolved = runGitInRepo(repoDir, ["rev-parse", "--git-path", gitPath]);
  return path.resolve(repoDir, resolved);
}

function gitDirPathInRepo(repoDir) {
  const resolved = runGitInRepo(repoDir, ["rev-parse", "--git-dir"]);
  return path.resolve(repoDir, resolved);
}

function gitCommonDirPathInRepo(repoDir) {
  const resolved = runGitInRepo(repoDir, ["rev-parse", "--git-common-dir"]);
  return path.resolve(repoDir, resolved);
}

function usesLinkedGitWorktree(repoDir) {
  return normalizeComparablePath(gitDirPathInRepo(repoDir)) !== normalizeComparablePath(gitCommonDirPathInRepo(repoDir));
}

function commonGitConfigPath(repoDir) {
  return path.join(gitCommonDirPathInRepo(repoDir), "config");
}

function worktreeConfigPath(repoDir) {
  return path.join(gitDirPathInRepo(repoDir), "config.worktree");
}

function managedGovExcludePath(repoDir) {
  if (usesLinkedGitWorktree(repoDir)) {
    return path.join(gitDirPathInRepo(repoDir), "info", "exclude");
  }
  return path.join(gitCommonDirPathInRepo(repoDir), "info", "exclude");
}

function commonInfoExcludePath(repoDir) {
  return path.join(gitCommonDirPathInRepo(repoDir), "info", "exclude");
}

function gitConfigValueOrEmpty(repoDir, args) {
  try {
    return runGitInRepo(repoDir, args);
  } catch {
    return "";
  }
}

function ensureWorktreeConfigEnabled(repoDir) {
  if (!usesLinkedGitWorktree(repoDir)) return;
  const configPath = commonGitConfigPath(repoDir);
  const current = gitConfigValueOrEmpty(repoDir, ["config", "--file", configPath, "--get", "extensions.worktreeConfig"]);
  if (String(current || "").trim().toLowerCase() === "true") return;
  execFileSync("git", ["config", "--file", configPath, "extensions.worktreeConfig", "true"], {
    cwd: repoDir,
    stdio: ["ignore", "ignore", "ignore"],
  });
}

function ensureManagedWorktreeExcludeConfig(repoDir, excludePath) {
  if (!usesLinkedGitWorktree(repoDir)) return;
  ensureWorktreeConfigEnabled(repoDir);
  const configPath = worktreeConfigPath(repoDir);
  fs.mkdirSync(path.dirname(configPath), { recursive: true });
  execFileSync("git", ["config", "--file", configPath, "core.excludesFile", excludePath], {
    cwd: repoDir,
    stdio: ["ignore", "ignore", "ignore"],
  });
}

function clearManagedWorktreeExcludeConfig(repoDir, excludePath) {
  if (!usesLinkedGitWorktree(repoDir)) return;
  const configPath = worktreeConfigPath(repoDir);
  if (!fs.existsSync(configPath)) return;
  const current = gitConfigValueOrEmpty(repoDir, ["config", "--file", configPath, "--get", "core.excludesFile"]);
  if (normalizeComparablePath(current) !== normalizeComparablePath(excludePath)) return;
  execFileSync("git", ["config", "--file", configPath, "--unset-all", "core.excludesFile"], {
    cwd: repoDir,
    stdio: ["ignore", "ignore", "ignore"],
  });
}

function readTextOrEmpty(filePath) {
  return fs.existsSync(filePath) ? fs.readFileSync(filePath, "utf8") : "";
}

function removeManagedGovExcludeRule(filePath) {
  if (!fs.existsSync(filePath)) return;
  const filtered = fs.readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .filter((line) => line !== SHARED_GOV_EXCLUDE_MARKER && line !== SHARED_GOV_EXCLUDE_RULE);
  const next = filtered.join("\n").replace(/\n+$/, "");
  fs.writeFileSync(filePath, next.length > 0 ? `${next}\n` : "", "utf8");
}

function repoDirUsesSharedGovKernelJunction(repoDir) {
  const repoAbs = path.resolve(repoDir);
  const govDir = path.join(repoAbs, ".GOV");
  const expectedGovKernelAbs = path.resolve(repoAbs, "..", "wt-gov-kernel", ".GOV");
  if (!fs.existsSync(govDir) || !fs.existsSync(expectedGovKernelAbs)) return false;

  const stat = fs.lstatSync(govDir);
  if (!stat.isSymbolicLink()) return false;

  try {
    const actualTarget = path.resolve(fs.realpathSync(govDir));
    const expectedTarget = path.resolve(fs.realpathSync(expectedGovKernelAbs));
    return normalizeComparablePath(actualTarget) === normalizeComparablePath(expectedTarget);
  } catch {
    return false;
  }
}

function isGovKernelWorktree(repoDir) {
  if (!GOV_KERNEL_WORKTREE_ABS) return false;
  return normalizeComparablePath(path.resolve(repoDir)) === normalizeComparablePath(GOV_KERNEL_WORKTREE_ABS);
}

function resolveWorktreeSpecForRepoDir(repoDir) {
  const repoAbs = path.resolve(repoDir);
  return WORKTREE_SPECS.find((entry) =>
    normalizeComparablePath(absFromRepo(entry.rel_path)) === normalizeComparablePath(repoAbs));
}

function trackedGovEntriesBuffer(repoDir) {
  return execFileSync("git", ["ls-files", "-z", "--", ".GOV"], {
    cwd: repoDir,
    encoding: "buffer",
    stdio: ["ignore", "pipe", "ignore"],
  });
}

function repoStatusLines(repoDir) {
  return runGitInRepo(repoDir, ["status", "--porcelain=v1", "--untracked-files=all"])
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

export function setGovTrackedPathsSkipWorktree(repoDir, enabled) {
  const trackedGovEntries = trackedGovEntriesBuffer(repoDir);
  if (!trackedGovEntries.length) return;

  execFileSync(
    "git",
    ["update-index", "-z", enabled ? "--skip-worktree" : "--no-skip-worktree", "--stdin"],
    {
      cwd: repoDir,
      input: trackedGovEntries,
      stdio: ["pipe", "ignore", "ignore"],
    },
  );
}

export function ensureGovWorktreeExclude(repoDir) {
  const excludePath = managedGovExcludePath(repoDir);
  const original = readTextOrEmpty(excludePath);
  if (original.includes(SHARED_GOV_EXCLUDE_MARKER)) return;

  const next = original.endsWith("\n") || original.length === 0
    ? `${original}${SHARED_GOV_EXCLUDE_MARKER}\n${SHARED_GOV_EXCLUDE_RULE}\n`
    : `${original}\n${SHARED_GOV_EXCLUDE_MARKER}\n${SHARED_GOV_EXCLUDE_RULE}\n`;
  fs.mkdirSync(path.dirname(excludePath), { recursive: true });
  fs.writeFileSync(excludePath, next, "utf8");
  ensureManagedWorktreeExcludeConfig(repoDir, excludePath);
  if (normalizeComparablePath(excludePath) !== normalizeComparablePath(commonInfoExcludePath(repoDir))) {
    removeManagedGovExcludeRule(commonInfoExcludePath(repoDir));
  }
}

export function clearGovWorktreeExclude(repoDir) {
  const excludePath = managedGovExcludePath(repoDir);
  removeManagedGovExcludeRule(excludePath);
  if (normalizeComparablePath(excludePath) !== normalizeComparablePath(commonInfoExcludePath(repoDir))) {
    removeManagedGovExcludeRule(commonInfoExcludePath(repoDir));
  }
  clearManagedWorktreeExcludeConfig(repoDir, excludePath);
}

export function suppressSharedGovJunctionDirt(repoDir) {
  if (!repoDirUsesSharedGovKernelJunction(repoDir)) {
    clearSharedGovJunctionSuppression(repoDir);
    return false;
  }
  ensureGovWorktreeExclude(repoDir);
  setGovTrackedPathsSkipWorktree(repoDir, true);
  return true;
}

export function clearSharedGovJunctionSuppression(repoDir) {
  clearGovWorktreeExclude(repoDir);
  setGovTrackedPathsSkipWorktree(repoDir, false);
}

export function inspectGovTrackingMode(repoDir) {
  const repoAbs = path.resolve(repoDir);
  const worktreeSpec = resolveWorktreeSpecForRepoDir(repoAbs);
  const sharedGovJunction = repoDirUsesSharedGovKernelJunction(repoAbs);
  return {
    repoDir: repoAbs,
    worktreeId: worktreeSpec?.id || "",
    worktreeRole: worktreeSpec?.role || "",
    sharedGovJunction,
    tracksGov: !sharedGovJunction,
    mode: sharedGovJunction ? "SUPPRESS_SHARED_GOV" : "TRACK_GOV",
    govKernelWorktree: isGovKernelWorktree(repoAbs),
  };
}

export function normalizeGovTrackingMode(repoDir) {
  const repoAbs = path.resolve(repoDir);
  if (repoDirUsesSharedGovKernelJunction(repoAbs)) {
    suppressSharedGovJunctionDirt(repoAbs);
    return inspectGovTrackingMode(repoAbs);
  }
  clearSharedGovJunctionSuppression(repoAbs);
  return inspectGovTrackingMode(repoAbs);
}

export function normalizePermanentGovTracking() {
  return WORKTREE_SPECS.map((spec) => {
    const repoAbs = absFromRepo(spec.rel_path);
    if (!fs.existsSync(repoAbs) || !gitCheckoutExists(repoAbs)) {
      return {
        repoDir: repoAbs,
        worktreeId: spec.id,
        worktreeRole: spec.role,
        exists: false,
        sharedGovJunction: false,
        tracksGov: false,
        mode: "MISSING",
      };
    }
    return {
      ...normalizeGovTrackingMode(repoAbs),
      exists: true,
    };
  });
}

export function ensureGovKernelTracksGov(repoDir) {
  if (isGovKernelWorktree(repoDir) || !repoDirUsesSharedGovKernelJunction(repoDir)) {
    clearSharedGovJunctionSuppression(repoDir);
    return { normalized: true, sharedGovJunction: false };
  }
  return { normalized: false, sharedGovJunction: true };
}

function settleCleanRepoState(repoDir) {
  let lastStatusLines = repoStatusLines(repoDir);
  if (lastStatusLines.length === 0) return { clean: true, statusLines: [] };

  for (let attempt = 1; attempt <= CLEAN_SETTLE_ATTEMPTS; attempt += 1) {
    suppressSharedGovJunctionDirt(repoDir);
    try {
      runGitInRepo(repoDir, ["update-index", "-q", "--refresh"]);
    } catch {
      // Continue: git status remains the source of truth for the final verdict.
    }
    lastStatusLines = repoStatusLines(repoDir);
    if (lastStatusLines.length === 0) {
      return { clean: true, statusLines: [] };
    }
    if (attempt < CLEAN_SETTLE_ATTEMPTS) {
      sleepSync(CLEAN_SETTLE_DELAY_MS);
    }
  }

  return { clean: false, statusLines: lastStatusLines };
}

function main() {
  const { worktreeId, approval, label } = parseArgs();
  requireApproval(worktreeId, approval);

  const spec = WORKTREE_SPECS.find((entry) => entry.id === worktreeId);
  if (!spec || !RESEEDABLE_WORKTREE_ROLES.has(spec.role)) {
    fail("Unsupported worktree_id for reseed helper", [
      `worktree_id=${worktreeId}`,
      "supported=wt-ilja",
    ]);
  }

  const absDir = absFromRepo(spec.rel_path);
  if (!fs.existsSync(absDir) || !gitCheckoutExists(absDir)) {
    fail("Target worktree is missing or is not a git checkout", [`path=${absDir}`]);
  }

  clearSharedGovJunctionSuppression(absDir);

  if (dirtyOutsideGovInRepo(absDir)) {
    fail("Refusing to reseed a dirty worktree", [
      `path=${absDir}`,
      "Commit, stash, or recover the non-.GOV changes first.",
    ]);
  }

  const originalBranch = currentBranchInRepo(absDir);
  if (originalBranch && ![spec.local_branch, "main"].includes(originalBranch)) {
    fail("Refusing to reseed while the worktree is checked out to an unexpected branch", [
      `path=${absDir}`,
      `current_branch=${originalBranch}`,
      `expected=${spec.local_branch} or main`,
    ]);
  }

  runGitInherit(absDir, ["fetch", "origin"]);

  if (!localBranchExists(absDir, spec.local_branch)) {
    fail("Permanent role/user branch not found locally", [
      `path=${absDir}`,
      `branch=${spec.local_branch}`,
    ]);
  }

  console.log(
    `[RESEED_PERMANENT_WORKTREE_FROM_MAIN] safety push origin/${spec.local_branch} <= ${spec.local_branch}`,
  );
  runGitInherit(absDir, ["push", "-u", "origin", `${spec.local_branch}:${spec.local_branch}`]);

  createSafetySnapshot(worktreeId, label);

  if (originalBranch === "main") {
    runGitInherit(absDir, ["checkout", spec.local_branch]);
  }

  if (!refExists(absDir, "refs/remotes/origin/main")) {
    fail("Remote canonical branch not found", [
      `path=${absDir}`,
      "expected=refs/remotes/origin/main",
    ]);
  }

  const detachedGovLink = detachExternalGovLink(absDir);
  if (detachedGovLink.detached) {
    console.log(
      `[RESEED_PERMANENT_WORKTREE_FROM_MAIN] detached external .GOV link before branch reset -> ${detachedGovLink.targetAbs}`,
    );
  }

  console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] resetting local main to origin/main in ${absDir}`);
  runGitInherit(absDir, ["branch", "-f", "main", "origin/main"]);

  console.log(
    `[RESEED_PERMANENT_WORKTREE_FROM_MAIN] reseeding ${spec.local_branch} from local main in ${absDir}`,
  );
  runGitInherit(absDir, ["checkout", "-B", spec.local_branch, "main"]);
  runGitInherit(absDir, ["branch", "--set-upstream-to", spec.remote_branch, spec.local_branch]);

  ensureGovJunction(absDir);
  suppressSharedGovJunctionDirt(absDir);

  const cleanSettle = settleCleanRepoState(absDir);
  if (!cleanSettle.clean || dirtyInRepo(absDir)) {
    fail("Worktree is dirty after reseed", [
      `path=${absDir}`,
      `settle_attempts=${CLEAN_SETTLE_ATTEMPTS}`,
      `settle_delay_ms=${CLEAN_SETTLE_DELAY_MS}`,
      `status_sample=${cleanSettle.statusLines.slice(0, 12).join(" | ") || "<none>"}`,
      "Expected a fully clean worktree after branch reset, .GOV junction repair, and local .GOV suppression.",
    ]);
  }

  console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] reseed complete: ${worktreeId}`);
  console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] local main base: origin/main`);
  console.log(
    `[RESEED_PERMANENT_WORKTREE_FROM_MAIN] backup branch preserved remotely at origin/${spec.local_branch} before local reseed`,
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
