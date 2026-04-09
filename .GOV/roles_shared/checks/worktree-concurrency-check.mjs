import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { EXECUTION_OWNER_RANGE_HELP } from "../scripts/session/session-policy.mjs";
import { loadSessionRegistry } from "../scripts/session/session-registry-lib.mjs";
import { evaluateSessionGovernanceState } from "../scripts/session/session-governance-state-lib.mjs";
import { loadPacket } from "../scripts/lib/role-resume-utils.mjs";
import { evaluateWpDeclaredTopology } from "../scripts/lib/wp-declared-topology-lib.mjs";
import { REPO_ROOT, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("worktree-concurrency-check.mjs", { role: "SHARED" });

const TASK_BOARD_PATH = ".GOV/roles_shared/records/TASK_BOARD.md";
const ACTIVE_SESSION_RUNTIME_STATES = new Set([
  "PLUGIN_REQUESTED",
  "TERMINAL_COMMAND_DISPATCHED",
  "PLUGIN_CONFIRMED",
  "CLI_ESCALATION_READY",
  "CLI_ESCALATION_USED",
  "STARTING",
  "READY",
  "COMMAND_RUNNING",
  "ACTIVE",
  "WAITING",
]);

function runGit(args) {
  return execFileSync("git", ["-C", REPO_ROOT, ...args], { stdio: "pipe" }).toString().trim();
}

function fail(message, details = []) {
  failWithMemory("worktree-concurrency-check.mjs", message, { role: "SHARED", details });
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

function isDirectExecution() {
  const entry = process.argv[1];
  if (!entry) return false;
  return path.resolve(entry) === fileURLToPath(import.meta.url);
}

export function wpRequiresDedicatedWorktreeMapping({ role = "", wpId = "" } = {}) {
  const normalizedRole = String(role || "").trim().toUpperCase();
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId.startsWith("WP-")) return false;
  if (normalizedRole === "MEMORY_MANAGER") return false;
  if (/^WP-MEMORY-HYGIENE_/i.test(normalizedWpId)) return false;
  return true;
}

export function collectWpIdsRequiringDedicatedWorktrees({
  inProgressWpIds = [],
  sessions = [],
  repoRoot,
} = {}) {
  const activeSessionWpIds = new Set();
  for (const session of sessions || []) {
    const runtimeState = String(session?.runtime_state || "").trim().toUpperCase();
    if (!ACTIVE_SESSION_RUNTIME_STATES.has(runtimeState)) continue;
    const governance = evaluateSessionGovernanceState(repoRoot, session);
    if (!governance.launchAllowed) continue;
    if (!wpRequiresDedicatedWorktreeMapping({ role: session?.role, wpId: governance.wpId })) continue;
    activeSessionWpIds.add(governance.wpId);
  }

  return [...new Set([
    ...(inProgressWpIds || []).filter((wpId) => wpRequiresDedicatedWorktreeMapping({ wpId })),
    ...activeSessionWpIds,
  ])];
}

function main() {
  // Local guard only; CI clones cannot/should not be required to have worktrees.
  if (process.env.CI || process.env.GITHUB_ACTIONS) {
    console.log("worktree-concurrency-check ok (skipped in CI)");
    return;
  }

  if (!fs.existsSync(repoPathAbs(TASK_BOARD_PATH))) {
    fail("Missing task board", [`Expected: ${TASK_BOARD_PATH}`]);
  }

  const taskBoard = fs.readFileSync(repoPathAbs(TASK_BOARD_PATH), "utf8");
  const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
  const inProgressWpIds = listInProgressWps(taskBoard);
  const { registry } = loadSessionRegistry(repoRoot);
  const wpIdsToCheck = collectWpIdsRequiringDedicatedWorktrees({
    inProgressWpIds,
    sessions: registry.sessions || [],
    repoRoot,
  });
  if (wpIdsToCheck.length === 0) {
    console.log("worktree-concurrency-check ok");
    return;
  }

  const worktrees = parseWorktreeList();
  const violations = [];
  const matchedPaths = new Map();

  for (const wpId of wpIdsToCheck) {
    let packetContent = "";
    try {
      packetContent = loadPacket(wpId);
    } catch {
      violations.push(`${wpId}: official packet missing or unreadable`);
      continue;
    }

    const topology = evaluateWpDeclaredTopology({
      repoRoot,
      wpId,
      packetContent,
      worktrees,
    });
    if (!topology.ok) {
      for (const issue of topology.issues) {
        violations.push(`${wpId}: ${issue}`);
      }
    }

    const normalizedPath = topology.topology.allowedSpecificPaths[0] || "";
    const owner = matchedPaths.get(normalizedPath);
    if (normalizedPath && owner && owner !== wpId) {
      violations.push(`${wpId}: shares declared coder worktree with ${owner} (one WP must map to one worktree)`);
    } else if (normalizedPath) {
      matchedPaths.set(normalizedPath, wpId);
    }

    // Worktree budget: max 1 WP-specific worktree per WP (coder + WP validator share it).
    // Integration Validator operates from handshake_main/main.
    // WP Validator now shares the coder worktree; no separate wtv-* worktree required.
    const MAX_WP_WORKTREES = 1;
    const wpSpecificWorktrees = topology.relatedWorktrees.filter((entry) => normalizeBranch(entry.branch) !== "main");
    if (wpSpecificWorktrees.length > MAX_WP_WORKTREES) {
      violations.push(
        `${wpId}: ${wpSpecificWorktrees.length} WP-specific worktrees found (max ${MAX_WP_WORKTREES}; coder + WP validator share one worktree). `
        + `Active: ${wpSpecificWorktrees.map((w) => normalizeBranch(w.branch) || `detached:${w.head || "<unknown>"}`).join(", ")}. `
        + "Reuse existing worktrees or clean up superseded ones before creating more.",
      );
    }
  }

  if (violations.length > 0) {
    fail("Concurrent WPs require dedicated per-WP worktree mappings (per protocols).", [
      `Tracked WPs: ${wpIdsToCheck.length}`,
      ...violations,
    ]);
  }

  console.log("worktree-concurrency-check ok");
}

if (isDirectExecution()) {
  main();
}
