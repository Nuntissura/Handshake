#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  REPO_ROOT,
  WORKSPACE_ROOT,
  currentBranchInRepo,
  dirtyInRepo,
  gitCheckoutExists,
  runGitInRepo,
} from "./git-topology-lib.mjs";

const PROTECTED_WORKTREES = new Set(["handshake_main", "wt-ilja", "wt-orchestrator", "wt-validator"]);

function fail(message, details = []) {
  console.error(`[DELETE_LOCAL_WORKTREE] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail("Usage: node .GOV/scripts/delete-local-worktree.mjs <WORKTREE_ID> --approve \"APPROVE DELETE LOCAL WORKTREE <WORKTREE_ID>\"", [
    "Example: node .GOV/scripts/delete-local-worktree.mjs wt-WP-1-Example --approve \"APPROVE DELETE LOCAL WORKTREE wt-WP-1-Example\"",
  ]);
}

function parseArgs() {
  const worktreeId = (process.argv[2] || "").trim();
  if (!worktreeId) usage();

  const args = process.argv.slice(3);
  const approveIndex = args.indexOf("--approve");
  const approval = approveIndex >= 0 ? String(args[approveIndex + 1] || "").trim() : "";
  if (!approval) usage();

  return { worktreeId, approval };
}

function requireApproval(worktreeId, approval) {
  const required = `APPROVE DELETE LOCAL WORKTREE ${worktreeId}`;
  if (!approval.includes(required)) {
    fail("Missing deterministic Operator approval text", [`required token: ${required}`]);
  }
}

function listRegisteredWorktrees() {
  const output = runGitInRepo(REPO_ROOT, ["worktree", "list", "--porcelain"]);
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

function createSafetySnapshot(worktreeId) {
  const label = `pre-delete-${worktreeId}`;
  execFileSync(process.execPath, [path.join(REPO_ROOT, ".GOV/scripts/backup-snapshot.mjs"), "--label", label], {
    cwd: REPO_ROOT,
    stdio: "inherit",
  });
}

function main() {
  const { worktreeId, approval } = parseArgs();
  requireApproval(worktreeId, approval);

  if (PROTECTED_WORKTREES.has(worktreeId)) {
    fail("Refusing to delete a protected worktree", [`worktree_id=${worktreeId}`]);
  }

  const absDir = path.resolve(WORKSPACE_ROOT, worktreeId);
  if (path.resolve(path.dirname(absDir)).toLowerCase() !== path.resolve(WORKSPACE_ROOT).toLowerCase()) {
    fail("Resolved target is not a direct child of the shared worktree root", [
      `worktree_id=${worktreeId}`,
      `resolved_path=${absDir}`,
    ]);
  }

  if (!fs.existsSync(absDir)) {
    fail("Worktree directory not found", [`path=${absDir}`]);
  }

  if (!gitCheckoutExists(absDir)) {
    fail("Target is not a git checkout; direct filesystem deletion is forbidden", [
      `path=${absDir}`,
      "Do not use Remove-Item/rm/del as a fallback. Manual operator recovery is required.",
    ]);
  }

  const registered = listRegisteredWorktrees();
  const worktreeRow = registered.find((row) => row.absPath.toLowerCase() === absDir.toLowerCase());
  if (!worktreeRow) {
    fail("Target is not a git-registered worktree for this repo; refusing deletion", [
      `path=${absDir}`,
      "Do not delete sibling directories directly. Inspect git/worktree state manually.",
    ]);
  }

  if (dirtyInRepo(absDir)) {
    fail("Refusing to delete a dirty worktree", [
      `path=${absDir}`,
      "Commit, stash, or recover the changes first. Cleanup must not destroy dirty state.",
    ]);
  }

  const currentBranch = currentBranchInRepo(absDir);
  if (currentBranch && ["main", "user_ilja", "role_orchestrator", "role_validator"].includes(currentBranch)) {
    fail("Refusing to delete a worktree checked out to a protected branch", [
      `path=${absDir}`,
      `branch=${currentBranch}`,
    ]);
  }

  createSafetySnapshot(worktreeId);

  try {
    execFileSync("git", ["worktree", "remove", absDir], {
      cwd: REPO_ROOT,
      stdio: "inherit",
    });
  } catch {
    fail("git worktree remove failed; cleanup is aborted", [
      `path=${absDir}`,
      "Do not attempt direct filesystem deletion. Stop and inspect git/worktree state.",
    ]);
  }

  if (fs.existsSync(absDir)) {
    fail("Worktree directory still exists after git worktree remove", [
      `path=${absDir}`,
      "Do not attempt manual deletion. Stop and inspect the repo state.",
    ]);
  }

  console.log(`[DELETE_LOCAL_WORKTREE] removed ${worktreeId}`);
}

main();
