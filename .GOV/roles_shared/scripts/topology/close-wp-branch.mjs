import { execFileSync } from "node:child_process";

import path from "node:path";

function runGit(args, opts = {}) {
  return execFileSync("git", args, { stdio: "pipe", ...opts }).toString().trim();
}

function runGitInherit(args) {
  execFileSync("git", args, { stdio: "inherit" });
}

function fail(message, details = []) {
  console.error(`[CLOSE_WP_BRANCH] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail("Usage: node .GOV/roles_shared/scripts/topology/close-wp-branch.mjs <WP_ID> [--remote] --approve \"<approval text>\"", [
    "Before running this helper, present the exact close action + target list to the Operator and capture `approved` or `proceed` for that list.",
    "Example (local only): node .GOV/roles_shared/scripts/topology/close-wp-branch.mjs WP-1-MEX-v1.2-Runtime-v3 --approve \"approved\"",
    "Example (also delete origin branch): node .GOV/roles_shared/scripts/topology/close-wp-branch.mjs WP-1-MEX-v1.2-Runtime-v3 --remote --approve \"proceed\"",
  ]);
}

function parseArgs() {
  const wpId = (process.argv[2] ?? "").trim();
  if (!wpId || !wpId.startsWith("WP-")) usage();
  const args = process.argv.slice(3);
  const remote = args.includes("--remote");
  const approveIndex = args.indexOf("--approve");
  const approval = approveIndex >= 0 ? (args[approveIndex + 1] ?? "").trim() : "";
  if (!approval) usage();
  return { wpId, remote, approval };
}

function branchForWp(wpId) {
  return `feat/${wpId}`;
}

function localBranchExists(branch) {
  try {
    execFileSync("git", ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`]);
    return true;
  } catch {
    return false;
  }
}

function isMergedIntoMain(branch) {
  try {
    execFileSync("git", ["merge-base", "--is-ancestor", branch, "main"]);
    return true;
  } catch {
    return false;
  }
}

function currentBranch() {
  return runGit(["branch", "--show-current"]);
}

function worktreeUsesBranch(branch) {
  const out = runGit(["worktree", "list", "--porcelain"]);
  const needle = `branch refs/heads/${branch}`;
  return out.split(/\r?\n/).some((line) => line.trim() === needle);
}

function remoteBranchExists(remoteName, branch) {
  try {
    const out = runGit(["ls-remote", "--heads", remoteName, branch]);
    return out.length > 0;
  } catch {
    return false;
  }
}

function requireApproval(approval, branch, remote) {
  const normalized = String(approval || "").trim().toLowerCase();
  if (normalized === "approved" || normalized === "proceed") return;
  fail("Missing valid approval acknowledgement", [
    `branch=${branch}`,
    `remote_delete=${remote ? "YES" : "NO"}`,
    "accepted approvals: approved | proceed",
  ]);
}

function createSafetySnapshot(wpId) {
  const label = `pre-close-${wpId}`;
  execFileSync(process.execPath, [path.join(".GOV", "scripts", "topology", "backup-snapshot.mjs"), "--label", label], {
    stdio: "inherit",
  });
}

function main() {
  const { wpId, remote, approval } = parseArgs();
  const branch = branchForWp(wpId);

  requireApproval(approval, branch, remote);

  if (!localBranchExists(branch)) {
    fail("Local WP branch not found", [`branch=${branch}`]);
  }

  if (currentBranch() === branch) {
    fail("Cannot delete the currently checked-out branch", [
      `branch=${branch}`,
      "Checkout main first.",
    ]);
  }

  if (worktreeUsesBranch(branch)) {
    fail("A git worktree is still using this branch", [
      `branch=${branch}`,
      "Remove/move that worktree before closing the WP branch.",
    ]);
  }

  if (!isMergedIntoMain(branch)) {
    fail("WP branch is not merged into main; refusing to delete", [
      `branch=${branch}`,
      "Merge it into main first, or pass `--force` (not supported).",
    ]);
  }

  createSafetySnapshot(wpId);

  // The upstream safety check in `git branch -d` can block deletion even when the branch
  // is already merged into `main`. We already proved ancestry, so force-delete the pointer.
  runGitInherit(["branch", "-D", branch]);

  if (remote) {
    const remoteName = "origin";
    if (!remoteBranchExists(remoteName, branch)) {
      console.warn(`[CLOSE_WP_BRANCH] Remote branch not found; skipping: ${remoteName}/${branch}`);
      return;
    }
    runGitInherit(["push", remoteName, "--delete", branch]);
  }
}

main();
