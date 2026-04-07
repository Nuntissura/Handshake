#!/usr/bin/env node

import path from "node:path";
import { execFileSync } from "node:child_process";
import { GOV_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const debugMode = process.argv.includes("--debug");
const sessionControlEnv = {
  ...process.env,
  ...(debugMode ? { HANDSHAKE_SESSION_CONTROL_DEBUG: "1" } : {}),
};

if (!["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(role) || !/^WP-/.test(wpId)) {
  console.error(
    `[SESSION_CONTROL] Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/session-control-cancel.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID> [--debug]`
  );
  process.exit(1);
}

execFileSync(
  process.execPath,
  [path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-command.mjs"), "CANCEL_SESSION", role, wpId],
  { stdio: "inherit", env: sessionControlEnv },
);
