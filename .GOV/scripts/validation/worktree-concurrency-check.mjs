import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const TASK_BOARD_PATH = ".GOV/roles_shared/TASK_BOARD.md";
const ORCH_GATES_PATH = ".GOV/roles/orchestrator/ORCHESTRATOR_GATES.json";

function runGit(args) {
  return execFileSync("git", args, { stdio: "pipe" }).toString().trim();
}

function fail(message, details = []) {
  console.error(`[WORKTREE_CONCURRENCY_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function listInProgressWps(taskBoard) {
  const re = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[IN_PROGRESS\]\s*$/;
  const wpIds = [];
  for (const line of taskBoard.split(/\r?\n/)) {
    const m = line.match(re);
    if (m) wpIds.push(m[1]);
  }
  return wpIds;
}

function parseWorktreeList() {
  const out = runGit(["worktree", "list", "--porcelain"]);
  const entries = [];
  let current = null;

  for (const line of out.split(/\r?\n/)) {
    if (!line.trim()) {
      if (current) entries.push(current);
      current = null;
      continue;
    }
    if (line.startsWith("worktree ")) {
      if (current) entries.push(current);
      current = { path: line.slice("worktree ".length).trim(), branch: "" };
      continue;
    }
    if (!current) continue;
    if (line.startsWith("branch ")) {
      current.branch = line.slice("branch ".length).trim();
    }
  }

  if (current) entries.push(current);
  return entries;
}

function normalizeBranch(branch) {
  return (branch || "").replace(/^refs\/heads\//, "").trim();
}

function isAbsoluteWorktreeDir(worktreeDir) {
  if (!worktreeDir) return false;
  const value = worktreeDir.trim();
  if (path.isAbsolute(value)) return true;
  if (/^[A-Za-z]:[\\/]/.test(value)) return true;
  if (value.startsWith("\\\\") || value.startsWith("//")) return true;
  return false;
}

function samePath(a, b) {
  return path.resolve(a).toLowerCase() === path.resolve(b).toLowerCase();
}

function loadPrepareMap() {
  const map = new Map();
  if (!fs.existsSync(ORCH_GATES_PATH)) return map;

  try {
    const gates = JSON.parse(fs.readFileSync(ORCH_GATES_PATH, "utf8"));
    const logs = Array.isArray(gates?.gate_logs) ? gates.gate_logs : [];
    for (const log of logs) {
      if (log?.type !== "PREPARE") continue;
      if (!log?.wpId || typeof log.wpId !== "string") continue;
      map.set(log.wpId, log);
    }
  } catch {
    return map;
  }

  return map;
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
const inProgressWpIds = listInProgressWps(taskBoard);
if (inProgressWpIds.length === 0) {
  console.log("worktree-concurrency-check ok");
  process.exit(0);
}

const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
const worktrees = parseWorktreeList();
const prepares = loadPrepareMap();
const violations = [];
const matchedPaths = new Map();

for (const wpId of inProgressWpIds) {
  const prepare = prepares.get(wpId) || null;
  const expectedBranch = normalizeBranch(prepare?.branch || `feat/${wpId}`);
  const worktree = worktrees.find((entry) => normalizeBranch(entry.branch) === expectedBranch);

  if (!worktree) {
    violations.push(
      `${wpId}: no linked worktree found for expected branch ${expectedBranch} (run: just worktree-add ${wpId} && just record-prepare ${wpId} {Coder-A|Coder-B})`,
    );
    continue;
  }

  if (prepare?.worktree_dir) {
    if (isAbsoluteWorktreeDir(prepare.worktree_dir)) {
      violations.push(
        `${wpId}: PREPARE.worktree_dir must be repo-relative, got absolute path: ${prepare.worktree_dir}`,
      );
    } else {
      const expectedPath = path.resolve(repoRoot, prepare.worktree_dir);
      if (!samePath(worktree.path, expectedPath)) {
        violations.push(
          `${wpId}: PREPARE.worktree_dir mismatch (expected ${prepare.worktree_dir} -> ${expectedPath}, git has ${worktree.path})`,
        );
      }
    }
  }

  const normalizedPath = path.resolve(worktree.path).toLowerCase();
  const owner = matchedPaths.get(normalizedPath);
  if (owner && owner !== wpId) {
    violations.push(`${wpId}: shares worktree ${worktree.path} with ${owner} (one WP must map to one worktree)`);
  } else {
    matchedPaths.set(normalizedPath, wpId);
  }
}

if (violations.length > 0) {
  fail("Concurrent WPs require dedicated per-WP worktree mappings (per protocols).", [
    `In Progress WPs: ${inProgressWpIds.length}`,
    ...violations,
  ]);
}

console.log("worktree-concurrency-check ok");
