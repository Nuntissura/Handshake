import { execFileSync } from "node:child_process";
import path from "node:path";
import {
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
} from "./session-policy.mjs";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const branchArg = String(process.argv[4] || "").trim();
const dirArg = String(process.argv[5] || "").trim();

function fail(message) {
  console.error(`[ROLE_SESSION_WORKTREE_ADD] ${message}`);
  process.exit(1);
}

if (!wpId || !wpId.startsWith("WP-")) {
  fail("Usage: node .GOV/scripts/role-session-worktree-add.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID> [branch] [dir]");
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
    return {
      branch: defaultIntegrationValidatorBranch(workPacketId),
      dir: defaultIntegrationValidatorWorktreeDir(workPacketId),
    };
  }
  return null;
}

const defaults = defaultsForRole(role, wpId);
if (!defaults) {
  fail(`Unknown role: ${role}`);
}

const branch = branchArg || defaults.branch;
const dir = dirArg || defaults.dir;
const scriptPath = path.join(".GOV", "scripts", "worktree-add.mjs");

execFileSync(process.execPath, [scriptPath, wpId, "main", branch, dir], {
  stdio: "inherit",
});

console.log(`[ROLE_SESSION_WORKTREE_ADD] role=${role} branch=${branch} dir=${dir}`);
