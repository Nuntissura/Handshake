import fs from "node:fs";
import path from "node:path";

const TASK_BOARD_PATH = "docs/TASK_BOARD.md";
const WORKTREES_DIR = path.join(".git", "worktrees");

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
  if (!fs.existsSync(WORKTREES_DIR)) return 0;
  try {
    return fs
      .readdirSync(WORKTREES_DIR, { withFileTypes: true })
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
  fail("Concurrent WPs require git worktrees (per protocols).", [
    `In Progress WPs: ${inProgress}`,
    `Linked worktrees present: ${linkedWorktrees} (dir: ${WORKTREES_DIR})`,
    `Required linked worktrees: ${requiredLinkedWorktrees}`,
    `Create: just worktree-add WP-<ID> (or: git worktree add ..\\wt-WP-<ID> feat/WP-<ID>)`,
  ]);
}

console.log("worktree-concurrency-check ok");
