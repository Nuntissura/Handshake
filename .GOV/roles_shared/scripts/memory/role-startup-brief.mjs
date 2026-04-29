#!/usr/bin/env node

import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";
import { formatStartupBriefForRole, startupBriefPathForRole } from "./role-startup-brief-lib.mjs";

registerFailCaptureHook("role-startup-brief.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("role-startup-brief.mjs", message, { role: "SHARED", details });
}

const role = String(process.argv[2] || "").trim().toUpperCase();

if (!role || !startupBriefPathForRole(role)) {
  fail("Usage: node .GOV/roles_shared/scripts/memory/role-startup-brief.mjs <ROLE>", [
    "Valid roles: ORCHESTRATOR, CLASSIC_ORCHESTRATOR, ACTIVATION_MANAGER, CODER, WP_VALIDATOR, INTEGRATION_VALIDATOR, VALIDATOR, MEMORY_MANAGER",
  ]);
}

try {
  console.log(formatStartupBriefForRole(role));
} catch (error) {
  fail(error?.message || String(error));
}
