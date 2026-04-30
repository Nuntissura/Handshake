#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_ABS } from "../scripts/lib/runtime-paths.mjs";
import {
  formatProtectedWorktreeResolutionDiagnostics,
  resolveProtectedWorktree,
  toPosix,
} from "../scripts/topology/git-topology-lib.mjs";

const mainResolution = resolveProtectedWorktree("handshake_main");
const errors = [];

if (!mainResolution.ok) {
  errors.push("handshake_main worktree resolution failed");
  errors.push(...formatProtectedWorktreeResolutionDiagnostics(mainResolution));
}

const gov = (...segments) => path.join(GOV_ROOT_ABS, ...segments);
const main = (...segments) => path.join(mainResolution.absDir || "", ...segments);

const files = [
  gov("codex", "Handshake_Codex_v1.4.md"),
  main("AGENTS.md"),
  gov("README.md"),
  gov("roles", "README.md"),
  gov("roles_shared", "README.md"),
  gov("roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
  gov("roles", "orchestrator", "docs", "ORCHESTRATOR_STARTUP_BRIEF.md"),
  gov("roles", "classic_orchestrator", "CLASSIC_ORCHESTRATOR_PROTOCOL.md"),
  gov("roles", "classic_orchestrator", "docs", "CLASSIC_ORCHESTRATOR_STARTUP_BRIEF.md"),
  gov("roles", "activation_manager", "ACTIVATION_MANAGER_PROTOCOL.md"),
  gov("roles", "activation_manager", "docs", "ACTIVATION_MANAGER_STARTUP_BRIEF.md"),
  gov("roles", "coder", "CODER_PROTOCOL.md"),
  gov("roles", "coder", "docs", "CODER_STARTUP_BRIEF.md"),
  gov("roles", "wp_validator", "WP_VALIDATOR_PROTOCOL.md"),
  gov("roles", "wp_validator", "docs", "WP_VALIDATOR_STARTUP_BRIEF.md"),
  gov("roles", "integration_validator", "INTEGRATION_VALIDATOR_PROTOCOL.md"),
  gov("roles", "integration_validator", "docs", "INTEGRATION_VALIDATOR_STARTUP_BRIEF.md"),
  gov("roles", "validator", "VALIDATOR_PROTOCOL.md"),
  gov("roles", "validator", "docs", "VALIDATOR_STARTUP_BRIEF.md"),
  gov("roles", "memory_manager", "MEMORY_MANAGER_PROTOCOL.md"),
  gov("roles", "memory_manager", "docs", "MEMORY_MANAGER_STARTUP_BRIEF.md"),
  gov("roles_shared", "docs", "START_HERE.md"),
  gov("roles_shared", "docs", "STARTUP_BRIEF_SCHEMA.md"),
  gov("roles_shared", "docs", "SHARED_STARTUP_BRIEF.md"),
  gov("spec", "SPEC_CURRENT.md"),
  gov("roles_shared", "docs", "ARCHITECTURE.md"),
  gov("roles_shared", "docs", "RUNBOOK_DEBUG.md"),
  gov("roles_shared", "docs", "REPO_RESILIENCE.md"),
  gov("roles_shared", "docs", "TOOLING_GUARDRAILS.md"),
  gov("roles_shared", "docs", "DEPRECATION_SUNSET_PLAN.md"),
];

for (const file of files) {
  if (!fs.existsSync(file)) errors.push(`Missing: ${toPosix(file)}`);
}

if (errors.length > 0) {
  for (const error of errors) console.error(error);
  process.exit(1);
}

console.log("docs-check ok");
