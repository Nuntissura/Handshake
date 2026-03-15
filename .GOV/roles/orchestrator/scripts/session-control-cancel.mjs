#!/usr/bin/env node

import path from "node:path";
import { execFileSync } from "node:child_process";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();

execFileSync(
  process.execPath,
  [path.join(".GOV", "roles", "orchestrator", "scripts", "session-control-command.mjs"), "CANCEL_SESSION", role, wpId],
  { stdio: "inherit" },
);
