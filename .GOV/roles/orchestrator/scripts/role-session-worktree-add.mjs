import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import {
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import { GOV_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const branchArg = String(process.argv[4] || "").trim();
const dirArg = String(process.argv[5] || "").trim();

function fail(message) {
  console.error(`[ROLE_SESSION_WORKTREE_ADD] ${message}`);
  process.exit(1);
}

if (!wpId || !wpId.startsWith("WP-")) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/role-session-worktree-add.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID> [branch] [dir]`);
}

function defaultsForRole(roleName, workPacketId) {
  if (roleName === "CODER") {
    return {
      branch: defaultCoderBranch(workPacketId),
      dir: defaultCoderWorktreeDir(workPacketId),
    };
  }
  if (roleName === "WP_VALIDATOR") {
    return {
      branch: defaultWpValidatorBranch(workPacketId),
      dir: defaultWpValidatorWorktreeDir(workPacketId),
    };
  }
  if (roleName === "INTEGRATION_VALIDATOR") {
    // Integration validator operates from handshake_main on branch main [CX-212D].
    // No WP-specific worktree creation needed.
    return null;
  }
  return null;
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function loadPrepareBaseBranch(wpIdValue) {
  const gatesPath = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "runtime", "ORCHESTRATOR_GATES.json");
  if (!fs.existsSync(gatesPath)) return "";
  try {
    const parsed = JSON.parse(fs.readFileSync(gatesPath, "utf8"));
    const logs = Array.isArray(parsed?.gate_logs) ? parsed.gate_logs : [];
    const lastPrepare = [...logs].reverse().find((entry) => entry?.wpId === wpIdValue && entry?.type === "PREPARE") || null;
    return String(lastPrepare?.branch || "").trim();
  } catch {
    return "";
  }
}

function loadPacketBaseBranch(wpIdValue) {
  const packetPath = path.join(GOV_ROOT_REPO_REL, "task_packets", `${wpIdValue}.md`);
  if (!fs.existsSync(packetPath)) return "";
  try {
    const packetText = fs.readFileSync(packetPath, "utf8");
    const localBranch = parseSingleField(packetText, "LOCAL_BRANCH");
    return localBranch === "<pending>" ? "" : localBranch;
  } catch {
    return "";
  }
}

// WP Validator operates from the coder worktree — no separate worktree creation [CX-212D].
if (role === "WP_VALIDATOR") {
  const coderDir = defaultCoderWorktreeDir(wpId);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] WP Validator operates from the coder worktree [CX-212D].`);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] No WP-specific validator worktree creation needed.`);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] Next: cd "${coderDir}"`);
  process.exit(0);
}

// Integration validator operates from handshake_main — no worktree creation [CX-212D].
if (role === "INTEGRATION_VALIDATOR") {
  console.log(`[ROLE_SESSION_WORKTREE_ADD] Integration Validator operates from handshake_main on branch main [CX-212D].`);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] No WP-specific worktree creation needed.`);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] Next: cd "../handshake_main"`);
  process.exit(0);
}

const defaults = defaultsForRole(role, wpId);
if (!defaults) {
  fail(`Unknown role: ${role}`);
}

const branch = branchArg || defaults.branch;
const dir = dirArg || defaults.dir;
const baseBranch = role === "CODER"
  ? "main"
  : (loadPacketBaseBranch(wpId) || loadPrepareBaseBranch(wpId));
if (role !== "CODER" && !baseBranch) {
  fail(`Cannot create ${role} worktree for ${wpId}: missing PREPARE/packet coder branch to base validator checkout on.`);
}
const scriptPath = path.join(GOV_ROOT_REPO_REL, "roles_shared", "scripts", "topology", "worktree-add.mjs");

execFileSync(process.execPath, [scriptPath, wpId, baseBranch || "main", branch, dir], {
  stdio: "inherit",
});

console.log(`[ROLE_SESSION_WORKTREE_ADD] role=${role} base=${baseBranch || "main"} branch=${branch} dir=${dir}`);
