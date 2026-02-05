import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

function runGit(args) {
  return execFileSync("git", args, { stdio: "pipe" }).toString().trim();
}

function runGitInherit(args) {
  execFileSync("git", args, { stdio: "inherit" });
}

function fail(message) {
  console.error(`[WORKTREE_ADD] ${message}`);
  process.exit(1);
}

function isBranchPresent(branch) {
  try {
    execFileSync("git", ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`]);
    return true;
  } catch {
    return false;
  }
}

function main() {
  const wpId = process.argv[2]?.trim();
  if (!wpId) {
    fail(
      "Usage: node .GOV/scripts/worktree-add.mjs <WP_ID> [base=main] [branch=feat/WP_ID] [dir=../wt-WP_ID]"
    );
  }

  const base = (process.argv[3] ?? "main").trim() || "main";
  const branch = (process.argv[4] ?? "").trim() || `feat/${wpId}`;
  const dir = (process.argv[5] ?? "").trim() || path.join("..", `wt-${wpId}`);

  const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
  const absDir = path.resolve(repoRoot, dir);

  if (fs.existsSync(absDir)) {
    fail(`Target directory already exists: ${absDir}`);
  }

  const alreadyHaveBranch = isBranchPresent(branch);
  if (alreadyHaveBranch) {
    console.log(`[WORKTREE_ADD] Using existing branch: ${branch}`);
    runGitInherit(["worktree", "add", absDir, branch]);
  } else {
    console.log(`[WORKTREE_ADD] Creating branch ${branch} from ${base}`);
    runGitInherit(["worktree", "add", "-b", branch, absDir, base]);
  }

  console.log("");
  console.log(`[WORKTREE_ADD] Worktree ready: ${absDir}`);
  console.log(`[WORKTREE_ADD] Next: cd "${absDir}"`);
}

main();

