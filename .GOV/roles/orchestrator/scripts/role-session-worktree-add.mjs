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
import { GOV_ROOT_REPO_REL, REPO_ROOT, repoPathAbs, resolveOrchestratorGatesPath, resolveWorkPacketPath, WORK_PACKET_STORAGE_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import {
  formatProtectedWorktreeResolutionDiagnostics,
  resolveProtectedWorktree,
} from "../../../roles_shared/scripts/topology/git-topology-lib.mjs";
registerFailCaptureHook("role-session-worktree-add.mjs", { role: "ORCHESTRATOR" });

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const branchArg = String(process.argv[4] || "").trim();
const dirArg = String(process.argv[5] || "").trim();

function fail(message) {
  failWithMemory("role-session-worktree-add.mjs", message, { role: "ORCHESTRATOR" });
}

if (!wpId || !wpId.startsWith("WP-")) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/role-session-worktree-add.mjs <ACTIVATION_MANAGER|CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR|MEMORY_MANAGER> <WP_ID> [branch] [dir]; manual-compatible roles also include CLASSIC_ORCHESTRATOR and VALIDATOR where resolveRoleConfig allows them.`);
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
  const gatesPath = repoPathAbs(resolveOrchestratorGatesPath());
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
  const resolved = resolveWorkPacketPath(wpIdValue);
  const packetPath = resolved?.packetPath || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpIdValue}.md`);
  const packetAbsPath = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbsPath)) return "";
  try {
    const packetText = fs.readFileSync(packetAbsPath, "utf8");
    const localBranch = parseSingleField(packetText, "LOCAL_BRANCH");
    return localBranch === "<pending>" ? "" : localBranch;
  } catch {
    return "";
  }
}

function loadPacketMergeBaseSha(wpIdValue) {
  const resolved = resolveWorkPacketPath(wpIdValue);
  const packetPath = resolved?.packetPath || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpIdValue}.md`);
  const packetAbsPath = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbsPath)) return "";
  try {
    const packetText = fs.readFileSync(packetAbsPath, "utf8");
    const mergeBaseField = parseSingleField(packetText, "MERGE_BASE_SHA");
    const mergeBaseSha = String(mergeBaseField || "").match(/\b([a-f0-9]{40})\b/i)?.[1] || "";
    return mergeBaseSha;
  } catch {
    return "";
  }
}

// Integration validator operates from handshake_main — no worktree creation [CX-212D].
if (role === "ACTIVATION_MANAGER" || role === "MEMORY_MANAGER") {
  console.log(`[ROLE_SESSION_WORKTREE_ADD] ${role} operates from the current governance worktree on branch gov_kernel.`);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] No WP-specific worktree creation needed.`);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] Next: stay in the current worktree.`);
  process.exit(0);
}

if (role === "INTEGRATION_VALIDATOR") {
  const mainResolution = resolveProtectedWorktree("handshake_main", { repoRoot: REPO_ROOT });
  console.log(`[ROLE_SESSION_WORKTREE_ADD] Integration Validator operates from handshake_main on branch main [CX-212D].`);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] No WP-specific worktree creation needed.`);
  console.log(`[ROLE_SESSION_WORKTREE_ADD] Resolved main worktree: ${mainResolution.absDir || "<missing>"}`);
  if (!mainResolution.ok) {
    console.log(`[ROLE_SESSION_WORKTREE_ADD] Main worktree resolution is not ready:`);
    for (const line of formatProtectedWorktreeResolutionDiagnostics(mainResolution)) {
      console.log(`[ROLE_SESSION_WORKTREE_ADD]   ${line}`);
    }
  }
  console.log(`[ROLE_SESSION_WORKTREE_ADD] Next: cd "${mainResolution.absDir || "../handshake_main"}"`);
  process.exit(0);
}

const defaults = defaultsForRole(role, wpId);
if (!defaults) {
  fail(`Unknown role: ${role}`);
}

const branch = branchArg || defaults.branch;
const dir = dirArg || defaults.dir;
const coderBaseRef = loadPacketMergeBaseSha(wpId) || "main";
const baseBranch = role === "CODER"
  ? coderBaseRef
  : (loadPacketBaseBranch(wpId) || loadPrepareBaseBranch(wpId));
if (role !== "CODER" && !baseBranch) {
  fail(`Cannot create ${role} worktree for ${wpId}: missing PREPARE/packet coder branch to base validator checkout on.`);
}
const scriptPath = path.join(GOV_ROOT_REPO_REL, "roles_shared", "scripts", "topology", "worktree-add.mjs");

execFileSync(process.execPath, [repoPathAbs(scriptPath), wpId, baseBranch || "main", branch, dir], {
  cwd: REPO_ROOT,
  stdio: "inherit",
});

if (role === "WP_VALIDATOR" && branch === defaultCoderBranch(wpId)) {
  console.log(`[ROLE_SESSION_WORKTREE_ADD] WP_VALIDATOR shares coder worktree [CX-503G]: branch=${branch} dir=${dir}`);
} else {
  console.log(`[ROLE_SESSION_WORKTREE_ADD] role=${role} base=${baseBranch || "main"} branch=${branch} dir=${dir}`);
}
