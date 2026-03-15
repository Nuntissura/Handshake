#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  REPO_ROOT,
  WORKSPACE_ROOT,
  currentBranchInRepo,
  dirtyInRepo,
  gitCheckoutExists,
  headShaInRepo,
  runGitInRepo,
} from "./git-topology-lib.mjs";
import { SESSION_REGISTRY_FILE } from "../session/session-policy.mjs";

const PROTECTED_WORKTREES = new Set(["handshake_main", "wt-ilja", "wt-orchestrator", "wt-validator"]);
const WORKTREE_CLEANUP_TOKEN_SCHEMA = "hsk.worktree_cleanup_token@1";

function fail(message, details = []) {
  console.error(`[DELETE_LOCAL_WORKTREE] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail("Usage: node .GOV/roles_shared/scripts/topology/delete-local-worktree.mjs <WORKTREE_ID> --approve \"APPROVE DELETE LOCAL WORKTREE <WORKTREE_ID>\"", [
    "Example: node .GOV/roles_shared/scripts/topology/delete-local-worktree.mjs wt-WP-1-Example --approve \"APPROVE DELETE LOCAL WORKTREE wt-WP-1-Example\"",
  ]);
}

function parseArgs() {
  const worktreeId = (process.argv[2] || "").trim();
  if (!worktreeId) usage();

  const args = process.argv.slice(3);
  const options = {
    approval: "",
    approvalExact: "",
    expectedAbsPath: "",
    expectedBranch: "",
    expectedHead: "",
    requireMainContains: "",
    tokenFile: "",
    token: "",
    wpId: "",
    role: "",
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
    if (token === "--approval-exact") {
      options.approvalExact = next();
      continue;
    }
    if (token === "--expect-abs-path") {
      options.expectedAbsPath = next();
      continue;
    }
    if (token === "--expect-branch") {
      options.expectedBranch = next();
      continue;
    }
    if (token === "--expect-head") {
      options.expectedHead = next();
      continue;
    }
    if (token === "--require-main-contains") {
      options.requireMainContains = next();
      continue;
    }
    if (token === "--token-file") {
      options.tokenFile = next();
      continue;
    }
    if (token === "--token") {
      options.token = next();
      continue;
    }
    if (token === "--wp-id") {
      options.wpId = next();
      continue;
    }
    if (token === "--role") {
      options.role = next().toUpperCase();
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

  const approval = options.approval;
  if (!approval) usage();

  return { worktreeId, ...options };
}

function comparablePath(value) {
  return path.resolve(String(value || "")).replace(/\\/g, "/").toLowerCase();
}

function requireApproval(worktreeId, approval, approvalExact = "") {
  const required = approvalExact || `APPROVE DELETE LOCAL WORKTREE ${worktreeId}`;
  if (approvalExact) {
    if (approval !== required) {
      fail("Approval text does not exactly match the generated cleanup target", [`required=${required}`]);
    }
    return;
  }
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
  execFileSync(process.execPath, [path.join(REPO_ROOT, ".GOV/roles_shared/scripts/topology/backup-snapshot.mjs"), "--label", label], {
    cwd: REPO_ROOT,
    stdio: "inherit",
  });
}

function resolveGitDir(absDir) {
  const gitDir = runGitInRepo(absDir, ["rev-parse", "--git-dir"]);
  return path.resolve(absDir, gitDir);
}

function stashDirtyWorktree(absDir, worktreeId) {
  const message = `SAFETY: before delete local worktree ${worktreeId}`;
  try {
    execFileSync("git", ["-c", "core.longpaths=true", "stash", "push", "-u", "-m", message], {
      cwd: absDir,
      stdio: "inherit",
    });
  } catch {
    fail("Failed to create safety stash for dirty worktree", [
      `path=${absDir}`,
      `stash_message=${message}`,
    ]);
  }
}

function loadSessionRegistry() {
  const filePath = path.resolve(REPO_ROOT, SESSION_REGISTRY_FILE);
  if (!fs.existsSync(filePath)) return { sessions: [] };
  try {
    const parsed = JSON.parse(fs.readFileSync(filePath, "utf8"));
    return parsed && typeof parsed === "object" ? parsed : { sessions: [] };
  } catch {
    return { sessions: [] };
  }
}

function verifyExpectedPath(absDir, expectedAbsPath) {
  if (!expectedAbsPath) return;
  if (comparablePath(absDir) !== comparablePath(expectedAbsPath)) {
    fail("Resolved worktree path does not match the generated cleanup target", [
      `expected=${path.resolve(expectedAbsPath)}`,
      `actual=${absDir}`,
    ]);
  }
}

function verifyExpectedBranch(absDir, expectedBranch) {
  if (!expectedBranch) return;
  const currentBranch = currentBranchInRepo(absDir);
  if (currentBranch !== expectedBranch) {
    fail("Current branch does not match the generated cleanup target", [
      `expected=${expectedBranch}`,
      `actual=${currentBranch || "<detached>"}`,
    ]);
  }
}

function verifyExpectedHead(absDir, expectedHead) {
  if (!expectedHead) return;
  const head = headShaInRepo(absDir);
  if (head !== expectedHead) {
    fail("Current HEAD does not match the generated cleanup target", [
      `expected=${expectedHead}`,
      `actual=${head || "<missing>"}`,
    ]);
  }
}

function verifyMainContains(commitSha) {
  if (!commitSha) return;
  try {
    execFileSync("git", ["merge-base", "--is-ancestor", commitSha, "main"], {
      cwd: REPO_ROOT,
      stdio: "ignore",
    });
  } catch {
    fail("Canonical main does not contain the required target commit", [
      `required_commit=${commitSha}`,
      "Run cleanup only after merge-to-main is complete.",
    ]);
  }
}

function verifyRoleSessionNotRunning(wpId, role) {
  if (!wpId || !role) return;
  const registry = loadSessionRegistry();
  const sessions = Array.isArray(registry.sessions) ? registry.sessions : [];
  const session = sessions.find((row) => row?.wp_id === wpId && String(row?.role || "").toUpperCase() === role);
  if (!session) return;
  if (String(session.runtime_state || "").toUpperCase() === "COMMAND_RUNNING") {
    fail("Refusing to delete a worktree while its governed session still has a running command", [
      `wp_id=${wpId}`,
      `role=${role}`,
      `session_key=${String(session.session_key || "")}`,
    ]);
  }
}

function verifyCleanupToken({ absDir, worktreeId, tokenFile, token, wpId, role, expectedAbsPath, expectedBranch, expectedHead }) {
  if (!tokenFile && !token) return;
  if (!tokenFile || !token) {
    fail("Cleanup token validation requires both --token-file and --token", []);
  }

  const gitDir = resolveGitDir(absDir);
  const tokenAbs = path.resolve(tokenFile);
  const gitDirComparable = comparablePath(gitDir);
  const tokenComparable = comparablePath(tokenAbs);
  if (!(tokenComparable === gitDirComparable || tokenComparable.startsWith(`${gitDirComparable}/`))) {
    fail("Cleanup token file is not inside the target worktree git admin directory", [
      `git_dir=${gitDir}`,
      `token_file=${tokenAbs}`,
    ]);
  }

  if (!fs.existsSync(tokenAbs)) {
    fail("Cleanup token file not found", [`token_file=${tokenAbs}`]);
  }

  let payload = null;
  try {
    payload = JSON.parse(fs.readFileSync(tokenAbs, "utf8"));
  } catch {
    fail("Cleanup token file is not valid JSON", [`token_file=${tokenAbs}`]);
  }

  if (String(payload?.schema_id || "") !== WORKTREE_CLEANUP_TOKEN_SCHEMA) {
    fail("Cleanup token schema mismatch", [
      `token_file=${tokenAbs}`,
      `expected_schema=${WORKTREE_CLEANUP_TOKEN_SCHEMA}`,
      `actual_schema=${String(payload?.schema_id || "<missing>")}`,
    ]);
  }

  if (String(payload?.worktree_id || "") !== worktreeId) {
    fail("Cleanup token worktree_id mismatch", [
      `expected=${worktreeId}`,
      `actual=${String(payload?.worktree_id || "<missing>")}`,
    ]);
  }

  if (wpId && String(payload?.wp_id || "") !== wpId) {
    fail("Cleanup token wp_id mismatch", [
      `expected=${wpId}`,
      `actual=${String(payload?.wp_id || "<missing>")}`,
    ]);
  }

  if (role && String(payload?.role || "").toUpperCase() !== role) {
    fail("Cleanup token role mismatch", [
      `expected=${role}`,
      `actual=${String(payload?.role || "<missing>")}`,
    ]);
  }

  if (expectedAbsPath && comparablePath(payload?.expected_abs_path || "") !== comparablePath(expectedAbsPath)) {
    fail("Cleanup token expected path mismatch", [
      `expected=${path.resolve(expectedAbsPath)}`,
      `actual=${String(payload?.expected_abs_path || "<missing>")}`,
    ]);
  }

  if (expectedBranch && String(payload?.expected_branch || "") !== expectedBranch) {
    fail("Cleanup token expected branch mismatch", [
      `expected=${expectedBranch}`,
      `actual=${String(payload?.expected_branch || "<missing>")}`,
    ]);
  }

  if (expectedHead && String(payload?.expected_head_sha || "") !== expectedHead) {
    fail("Cleanup token expected HEAD mismatch", [
      `expected=${expectedHead}`,
      `actual=${String(payload?.expected_head_sha || "<missing>")}`,
    ]);
  }

  const expiresAt = String(payload?.expires_at || "").trim();
  if (!expiresAt || Number.isNaN(Date.parse(expiresAt))) {
    fail("Cleanup token expires_at is missing or invalid", [`token_file=${tokenAbs}`]);
  }
  if (Date.now() > Date.parse(expiresAt)) {
    fail("Cleanup token is expired", [
      `token_file=${tokenAbs}`,
      `expires_at=${expiresAt}`,
    ]);
  }

  const actualHash = crypto.createHash("sha256").update(String(token)).digest("hex");

  if (String(payload?.token_sha256 || "") !== actualHash) {
    fail("Cleanup token value does not match the issued worktree token", [
      `token_file=${tokenAbs}`,
    ]);
  }
}

function main() {
  const {
    worktreeId,
    approval,
    approvalExact,
    expectedAbsPath,
    expectedBranch,
    expectedHead,
    requireMainContains,
    tokenFile,
    token,
    wpId,
    role,
    precreatedSnapshotRoot,
    stashDirty,
  } = parseArgs();
  requireApproval(worktreeId, approval, approvalExact);

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

  verifyExpectedPath(absDir, expectedAbsPath);
  verifyExpectedBranch(absDir, expectedBranch);
  verifyExpectedHead(absDir, expectedHead);
  verifyMainContains(requireMainContains);
  verifyRoleSessionNotRunning(wpId, role);
  verifyCleanupToken({
    absDir,
    worktreeId,
    tokenFile,
    token,
    wpId,
    role,
    expectedAbsPath,
    expectedBranch,
    expectedHead,
  });

  if (dirtyInRepo(absDir)) {
    if (!stashDirty) {
      fail("Refusing to delete a dirty worktree", [
        `path=${absDir}`,
        "Commit, stash, or recover the changes first. Cleanup must not destroy dirty state.",
      ]);
    }
    stashDirtyWorktree(absDir, worktreeId);
    if (dirtyInRepo(absDir)) {
      fail("Worktree remains dirty after safety stash", [
        `path=${absDir}`,
        "Manual recovery is required before deletion.",
      ]);
    }
  }

  const currentBranch = currentBranchInRepo(absDir);
  if (currentBranch && ["main", "user_ilja", "role_orchestrator", "role_validator"].includes(currentBranch)) {
    fail("Refusing to delete a worktree checked out to a protected branch", [
      `path=${absDir}`,
      `branch=${currentBranch}`,
    ]);
  }

  if (precreatedSnapshotRoot) {
    const snapshotRoot = path.resolve(precreatedSnapshotRoot);
    if (!fs.existsSync(snapshotRoot)) {
      fail("Precreated snapshot root does not exist", [`snapshot_root=${snapshotRoot}`]);
    }
    console.log(`[DELETE_LOCAL_WORKTREE] using precreated snapshot ${snapshotRoot}`);
  } else {
    createSafetySnapshot(worktreeId);
  }

  try {
    execFileSync("git", ["-c", "core.longpaths=true", "worktree", "remove", absDir], {
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

