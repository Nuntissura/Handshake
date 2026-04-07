#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
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

const PROTECTED_WORKTREES = new Set(["handshake_main", "wt-ilja", "wt-gov-kernel"]);
const WORKTREE_CLEANUP_TOKEN_SCHEMA = "hsk.worktree_cleanup_token@1";

function fail(message, details = []) {
  console.error(`[DELETE_LOCAL_WORKTREE] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail("Usage: node .GOV/roles_shared/scripts/topology/delete-local-worktree.mjs <WORKTREE_ID> --approve \"approved|proceed\"", [
    "Before running this helper, present the exact cleanup action + target list to the Operator and capture `approved` or `proceed` for that list.",
    "Example: node .GOV/roles_shared/scripts/topology/delete-local-worktree.mjs wt-WP-1-Example --approve \"approved\"",
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
    allowProtectedWorktree: false,
    ignoreSharedGovJunctionDirt: false,
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
    if (token === "--allow-protected-worktree") {
      options.allowProtectedWorktree = true;
      continue;
    }
    if (token === "--ignore-shared-gov-junction-dirt") {
      options.ignoreSharedGovJunctionDirt = true;
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

function normalizeLinkedPath(linkPath, targetPath) {
  const raw = String(targetPath || "").trim().replace(/^\\\\\?\\/, "");
  if (!raw) return "";
  return path.resolve(path.dirname(linkPath), raw);
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

function normalizeApproval(value) {
  return String(value || "").trim().toLowerCase();
}

function requireApproval(worktreeId, approval, approvalExact = "") {
  const normalized = normalizeApproval(approval);
  if (normalized === "approved" || normalized === "proceed") return;

  fail("Missing valid approval acknowledgement", [
    "accepted approvals: approved | proceed",
    `worktree_id=${worktreeId}`,
  ]);
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

function listDirtyPaths(absDir) {
  const output = runGitInRepo(absDir, ["status", "--porcelain=v1"]);
  const lines = output.split(/\r?\n/).map((line) => line.trimEnd()).filter(Boolean);
  const paths = [];
  for (const line of lines) {
    const rawPath = line.slice(3).trim();
    if (!rawPath) continue;
    const resolvedPath = rawPath.includes("->")
      ? rawPath.split("->").pop().trim()
      : rawPath;
    paths.push(resolvedPath.replace(/\\/g, "/"));
  }
  return paths;
}

function isSharedGovPath(value) {
  const normalized = String(value || "")
    .replace(/\\/g, "/")
    .trim()
    .replace(/^"+|"+$/g, "");
  return normalized.startsWith(".GOV/") || normalized.startsWith("GOV/");
}

export function detachExternalGovLink(absDir) {
  const govDir = path.join(absDir, ".GOV");
  if (!fs.existsSync(govDir)) {
    return {
      detached: false,
      govDir,
      reason: "missing",
      targetAbs: "",
    };
  }

  let stat;
  try {
    stat = fs.lstatSync(govDir);
  } catch {
    return {
      detached: false,
      govDir,
      reason: "unreadable",
      targetAbs: "",
    };
  }

  if (!stat.isSymbolicLink()) {
    return {
      detached: false,
      govDir,
      reason: "not_linked",
      targetAbs: "",
    };
  }

  let targetAbs = "";
  try {
    targetAbs = normalizeLinkedPath(govDir, fs.readlinkSync(govDir));
  } catch {
    return {
      detached: false,
      govDir,
      reason: "readlink_failed",
      targetAbs: "",
    };
  }

  const worktreeComparable = comparablePath(absDir);
  const targetComparable = comparablePath(targetAbs);
  const insideWorktree = targetComparable === worktreeComparable
    || targetComparable.startsWith(`${worktreeComparable}/`);
  if (insideWorktree) {
    return {
      detached: false,
      govDir,
      reason: "linked_inside_worktree",
      targetAbs,
    };
  }

  removeDirectoryLinkOnly(govDir);
  return {
    detached: true,
    govDir,
    reason: "detached_external_link",
    targetAbs,
  };
}

function reducePathsForSelectiveStash(paths) {
  const reduced = new Set();
  for (const rawPath of paths) {
    const normalized = String(rawPath || "").replace(/\\/g, "/").trim();
    if (!normalized) continue;
    const top = normalized.split("/")[0];
    reduced.add(top || normalized);
  }
  return [...reduced].sort();
}

function stashSelectedPaths(absDir, worktreeId, dirtyPaths) {
  const selectedRoots = reducePathsForSelectiveStash(dirtyPaths);
  if (selectedRoots.length === 0) return;
  const message = `SAFETY: before delete local worktree ${worktreeId}`;
  try {
    execFileSync("git", ["-c", "core.longpaths=true", "stash", "push", "-u", "-m", message, "--", ...selectedRoots], {
      cwd: absDir,
      stdio: "inherit",
    });
  } catch {
    fail("Failed to create selective safety stash for dirty worktree", [
      `path=${absDir}`,
      `stash_message=${message}`,
      `selected_roots=${selectedRoots.join(", ")}`,
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
    allowProtectedWorktree,
    ignoreSharedGovJunctionDirt,
  } = parseArgs();
  requireApproval(worktreeId, approval, approvalExact);

  if (!allowProtectedWorktree && PROTECTED_WORKTREES.has(worktreeId)) {
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

  const dirtyPaths = listDirtyPaths(absDir);
  const blockingDirtyPaths = ignoreSharedGovJunctionDirt
    ? dirtyPaths.filter((entry) => !isSharedGovPath(entry))
    : dirtyPaths;

  if (blockingDirtyPaths.length > 0) {
    if (!stashDirty) {
      fail("Refusing to delete a dirty worktree", [
        `path=${absDir}`,
        "Commit, stash, or recover the changes first. Cleanup must not destroy dirty state.",
      ]);
    }
    if (ignoreSharedGovJunctionDirt) {
      stashSelectedPaths(absDir, worktreeId, blockingDirtyPaths);
    } else {
      stashDirtyWorktree(absDir, worktreeId);
    }

    const remainingDirtyPaths = listDirtyPaths(absDir);
    const remainingBlockingDirtyPaths = ignoreSharedGovJunctionDirt
      ? remainingDirtyPaths.filter((entry) => !isSharedGovPath(entry))
      : remainingDirtyPaths;
    if (remainingBlockingDirtyPaths.length > 0) {
      fail("Worktree remains dirty after safety stash", [
        `path=${absDir}`,
        "Manual recovery is required before deletion.",
      ]);
    }
  }

  const currentBranch = currentBranchInRepo(absDir);
  if (!allowProtectedWorktree && currentBranch && ["main", "user_ilja", "gov_kernel"].includes(currentBranch)) {
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

  const detachedGovLink = detachExternalGovLink(absDir);
  if (detachedGovLink.detached) {
    console.log(`[DELETE_LOCAL_WORKTREE] detached external .GOV link before removal -> ${detachedGovLink.targetAbs}`);
  }

  try {
    const removeArgs = ["-c", "core.longpaths=true", "worktree", "remove"];
    if (ignoreSharedGovJunctionDirt || detachedGovLink.detached) {
      removeArgs.push("--force");
    }
    removeArgs.push(absDir);
    execFileSync("git", removeArgs, {
      cwd: REPO_ROOT,
      stdio: "inherit",
    });
  } catch (removeError) {
    const message = String(removeError?.message || removeError || "");
    const isLongPath = /filename too long|name too long|ENAMETOOLONG/i.test(message);
    if (isLongPath && process.platform === "win32") {
      fail("git worktree remove failed due to Windows MAX_PATH (260 char) limit", [
        `path=${absDir}`,
        "RECOVERY: enable long paths system-wide via Windows Registry:",
        "  reg add HKLM\\SYSTEM\\CurrentControlSet\\Control\\FileSystem /v LongPathsEnabled /t REG_DWORD /d 1 /f",
        "  then restart the shell and retry this command.",
        "ALTERNATIVE: use robocopy to remove the deep directory tree:",
        "  mkdir empty_dir && robocopy empty_dir \"<target>\" /mir /r:1 /w:0 && rmdir empty_dir \"<target>\"",
        "Do NOT use rm -rf or force-delete as a workaround.",
      ]);
    }
    fail("git worktree remove failed; cleanup is aborted", [
      `path=${absDir}`,
      "Do not attempt direct filesystem deletion. Stop and inspect git/worktree state.",
    ]);
  }

  if (fs.existsSync(absDir)) {
    fail("Worktree directory still exists after git worktree remove", [
      `path=${absDir}`,
      "Do not attempt manual deletion. Stop and inspect the repo state.",
      ...(process.platform === "win32" ? [
        "HINT: if this is a Windows long-path issue, enable LongPathsEnabled in registry or use robocopy /mir with an empty directory.",
      ] : []),
    ]);
  }

  console.log(`[DELETE_LOCAL_WORKTREE] removed ${worktreeId}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
