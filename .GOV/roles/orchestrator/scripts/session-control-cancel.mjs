#!/usr/bin/env node

import path from "node:path";
import { execFileSync } from "node:child_process";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();

if (!["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(role) || !/^WP-/.test(wpId)) {
  console.error(
    "[SESSION_CONTROL] Usage: node .GOV/roles/orchestrator/scripts/session-control-cancel.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID>"
  );
  process.exit(1);
}

execFileSync(
  process.execPath,
  [path.join(".GOV", "roles", "orchestrator", "scripts", "session-control-command.mjs"), "CANCEL_SESSION", role, wpId],
  { stdio: "inherit" },
);
