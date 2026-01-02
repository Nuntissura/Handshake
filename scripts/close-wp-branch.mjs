import { execFileSync } from "node:child_process";

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
  fail("Usage: node scripts/close-wp-branch.mjs <WP_ID> [--remote]", [
    "Example (local only): node scripts/close-wp-branch.mjs WP-1-MEX-v1.2-Runtime-v3",
    "Example (also delete origin branch): node scripts/close-wp-branch.mjs WP-1-MEX-v1.2-Runtime-v3 --remote",
  ]);
}

function parseArgs() {
  const wpId = (process.argv[2] ?? "").trim();
  if (!wpId || !wpId.startsWith("WP-")) usage();
  const remote = process.argv.slice(3).includes("--remote");
  return { wpId, remote };
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

function main() {
  const { wpId, remote } = parseArgs();
  const branch = branchForWp(wpId);

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

  runGitInherit(["branch", "-d", branch]);

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

