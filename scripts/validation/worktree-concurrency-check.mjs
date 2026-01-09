import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const TASK_BOARD_PATH = "docs/TASK_BOARD.md";

function runGit(args) {
  return execFileSync("git", args, { stdio: "pipe" }).toString().trim();
}

function getWorktreesDir() {
  try {
    const commonDir = runGit(["rev-parse", "--git-common-dir"]);
    if (!commonDir) return null;
    return path.join(path.resolve(process.cwd(), commonDir), "worktrees");
  } catch {
    return null;
  }
}

function fail(message, details = []) {
  console.error(`[WORKTREE_CONCURRENCY_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function countInProgressWps(taskBoard) {
  const re = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[IN_PROGRESS\]\s*$/;
  return taskBoard.split(/\r?\n/).filter((line) => re.test(line)).length;
}

function countLinkedWorktrees() {
  const worktreesDir = getWorktreesDir();
  if (!worktreesDir) return 0;
  if (!fs.existsSync(worktreesDir)) return 0;
  try {
    return fs
      .readdirSync(worktreesDir, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .length;
  } catch {
    return 0;
  }
}

// Local guard only; CI clones cannot/should not be required to have worktrees.
if (process.env.CI || process.env.GITHUB_ACTIONS) {
  console.log("worktree-concurrency-check ok (skipped in CI)");
  process.exit(0);
}

if (!fs.existsSync(TASK_BOARD_PATH)) {
  fail("Missing task board", [`Expected: ${TASK_BOARD_PATH}`]);
}

const taskBoard = fs.readFileSync(TASK_BOARD_PATH, "utf8");
const inProgress = countInProgressWps(taskBoard);
const requiredLinkedWorktrees = Math.max(0, inProgress - 1);
const linkedWorktrees = countLinkedWorktrees();

if (linkedWorktrees < requiredLinkedWorktrees) {
  const worktreesDir = getWorktreesDir();
  fail("Concurrent WPs require git worktrees (per protocols).", [
    `In Progress WPs: ${inProgress}`,
    `Linked worktrees present: ${linkedWorktrees} (dir: ${worktreesDir ?? "(unknown)"})`,
    `Required linked worktrees: ${requiredLinkedWorktrees}`,
    `Create: just worktree-add WP-<ID> (or: git worktree add ..\\wt-WP-<ID> feat/WP-<ID>)`,
  ]);
}

console.log("worktree-concurrency-check ok");
