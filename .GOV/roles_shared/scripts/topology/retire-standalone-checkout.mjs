#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  WORKSPACE_ROOT,
  currentBranchInRepo,
  dirtyInRepo,
  gitCheckoutExists,
  runGitInRepo,
} from "./git-topology-lib.mjs";

const PROTECTED_STANDALONE_BRANCHES = new Set(["main", "user_ilja", "gov_kernel"]);
const DEFAULT_ARCHIVE_ROOT = path.resolve(
  WORKSPACE_ROOT,
  "gov_runtime",
  "roles_shared",
  "LEGACY_STANDALONE_CHECKOUT_ARCHIVE",
);

function fail(message, details = []) {
  console.error(`[RETIRE_STANDALONE_CHECKOUT] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail(
    "Usage: node .GOV/roles_shared/scripts/topology/retire-standalone-checkout.mjs <CHECKOUT_ID> --approve \"approved|proceed\" [--archive-root <path>] [--precreated-snapshot-root <path>] [--stash-dirty]",
    [
      "Present the exact retirement action + target list to the Operator and capture `approved` or `proceed` for that list before running this helper.",
      "Example: node .GOV/roles_shared/scripts/topology/retire-standalone-checkout.mjs wt-orchestrator --approve approved --stash-dirty",
    ],
  );
}

function parseArgs() {
  const checkoutId = String(process.argv[2] || "").trim();
  if (!checkoutId) usage();

  const args = process.argv.slice(3);
  const options = {
    approval: "",
    archiveRoot: "",
    precreatedSnapshotRoot: "",
    stashDirty: false,
  };

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
      options.approval = next();
      continue;
    }
    if (token === "--archive-root") {
      options.archiveRoot = next();
      continue;
    }
    if (token === "--precreated-snapshot-root") {
      options.precreatedSnapshotRoot = next();
      continue;
    }
    if (token === "--stash-dirty") {
      options.stashDirty = true;
      continue;
    }
    fail("Unknown argument", [`arg=${token}`]);
  }

  if (!options.approval) usage();
  return { checkoutId, ...options };
}

function requireApproval(checkoutId, approval) {
  const normalized = String(approval || "").trim().toLowerCase();
  if (normalized === "approved" || normalized === "proceed") return;
  fail("Missing valid approval acknowledgement", [
    "accepted approvals: approved | proceed",
    `checkout_id=${checkoutId}`,
  ]);
}

function comparablePath(value) {
  return path.resolve(String(value || "")).replace(/\\/g, "/").toLowerCase();
}

function listRegisteredWorktrees(repoDir) {
  const output = runGitInRepo(repoDir, ["worktree", "list", "--porcelain"]);
  const rows = [];
  let current = null;

  for (const raw of output.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) {
      if (current) rows.push(current);
      current = null;
      continue;
    }
    if (line.startsWith("worktree ")) {
      if (current) rows.push(current);
      current = { absPath: path.resolve(line.slice("worktree ".length).trim()) };
      continue;
    }
    if (!current) continue;
    if (line.startsWith("branch ")) current.branchRef = line.slice("branch ".length).trim();
    if (line === "detached") current.detached = true;
  }
  if (current) rows.push(current);
  return rows;
}

function createSafetySnapshot(checkoutId) {
  const label = `pre-retire-${checkoutId}`;
  execFileSync(process.execPath, [path.join(WORKSPACE_ROOT, "wt-gov-kernel", ".GOV/roles_shared/scripts/topology/backup-snapshot.mjs"), "--label", label], {
    cwd: path.resolve(WORKSPACE_ROOT, "wt-gov-kernel"),
    stdio: "inherit",
  });
}

function stashDirtyCheckout(absDir, checkoutId) {
  const message = `SAFETY: before retire standalone checkout ${checkoutId}`;
  try {
    execFileSync("git", ["-c", "core.longpaths=true", "stash", "push", "-u", "-m", message], {
      cwd: absDir,
      stdio: "inherit",
    });
  } catch {
    fail("Failed to create safety stash for dirty standalone checkout", [
      `path=${absDir}`,
      `stash_message=${message}`,
    ]);
  }
}

function ensureStandaloneRoot(absDir) {
  const topLevel = path.resolve(runGitInRepo(absDir, ["rev-parse", "--show-toplevel"]));
  if (comparablePath(topLevel) !== comparablePath(absDir)) {
    fail("Target is not the main checkout root of its git repository", [
      `expected=${absDir}`,
      `actual=${topLevel}`,
    ]);
  }

  const rows = listRegisteredWorktrees(absDir);
  const extraRows = rows.filter((row) => comparablePath(row.absPath) !== comparablePath(absDir));
  if (extraRows.length > 0) {
    fail("Standalone checkout still owns linked worktrees", extraRows.map((row) => `linked_worktree=${row.absPath}`));
  }
}

function ensureArchiveDestination(archiveRoot, checkoutId) {
  fs.mkdirSync(archiveRoot, { recursive: true });
  const timestamp = new Date().toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
  const destination = path.resolve(archiveRoot, `${timestamp}-${checkoutId}`);
  if (fs.existsSync(destination)) {
    fail("Archive destination already exists", [`destination=${destination}`]);
  }
  return destination;
}

function main() {
  const { checkoutId, approval, archiveRoot, precreatedSnapshotRoot, stashDirty } = parseArgs();
  requireApproval(checkoutId, approval);

  const absDir = path.resolve(WORKSPACE_ROOT, checkoutId);
  if (path.resolve(path.dirname(absDir)).toLowerCase() !== path.resolve(WORKSPACE_ROOT).toLowerCase()) {
    fail("Resolved target is not a direct child of the shared worktree root", [
      `checkout_id=${checkoutId}`,
      `resolved_path=${absDir}`,
    ]);
  }

  if (!fs.existsSync(absDir)) {
    fail("Standalone checkout directory not found", [`path=${absDir}`]);
  }
  if (!gitCheckoutExists(absDir)) {
    fail("Target is not a git checkout", [`path=${absDir}`]);
  }

  ensureStandaloneRoot(absDir);

  const currentBranch = currentBranchInRepo(absDir);
  if (PROTECTED_STANDALONE_BRANCHES.has(currentBranch)) {
    fail("Refusing to retire a protected standalone checkout", [
      `path=${absDir}`,
      `branch=${currentBranch}`,
    ]);
  }

  if (dirtyInRepo(absDir)) {
    if (!stashDirty) {
      fail("Refusing to retire a dirty standalone checkout", [
        `path=${absDir}`,
        "Commit, stash, or recover the changes first. Retirement must not destroy dirty state.",
      ]);
    }
    stashDirtyCheckout(absDir, checkoutId);
    if (dirtyInRepo(absDir)) {
      fail("Standalone checkout remains dirty after safety stash", [
        `path=${absDir}`,
        "Manual recovery is required before retirement.",
      ]);
    }
  }

  if (precreatedSnapshotRoot) {
    const snapshotRoot = path.resolve(precreatedSnapshotRoot);
    if (!fs.existsSync(snapshotRoot)) {
      fail("Precreated snapshot root does not exist", [`snapshot_root=${snapshotRoot}`]);
    }
    console.log(`[RETIRE_STANDALONE_CHECKOUT] using precreated snapshot ${snapshotRoot}`);
  } else {
    createSafetySnapshot(checkoutId);
  }

  const destination = ensureArchiveDestination(archiveRoot || DEFAULT_ARCHIVE_ROOT, checkoutId);
  try {
    fs.renameSync(absDir, destination);
  } catch (error) {
    fail("Failed to archive standalone checkout", [
      `source=${absDir}`,
      `destination=${destination}`,
      String(error?.message || error),
    ]);
  }

  if (fs.existsSync(absDir)) {
    fail("Standalone checkout source path still exists after archive move", [
      `source=${absDir}`,
      `destination=${destination}`,
    ]);
  }
  if (!fs.existsSync(destination)) {
    fail("Archived standalone checkout destination is missing after move", [
      `destination=${destination}`,
    ]);
  }

  console.log(`[RETIRE_STANDALONE_CHECKOUT] archived ${checkoutId}`);
  console.log(`[RETIRE_STANDALONE_CHECKOUT] archive_path=${destination}`);
}

main();
