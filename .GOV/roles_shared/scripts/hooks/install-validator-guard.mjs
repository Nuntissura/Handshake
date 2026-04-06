#!/usr/bin/env node
/**
 * Installs the validator write-guard as a Claude Code PreToolUse hook
 * in a coder worktree's .claude/settings.local.json.
 *
 * Usage: node install-validator-guard.mjs <WP_ID>
 *
 * The orchestrator should:
 *   1. Run this BEFORE launching the validator session.
 *   2. Remove the settings file (or the hook entry) BEFORE the coder resumes.
 *
 * RGF-105 — Mechanical Tool-Call Guards for Validator Sessions
 */

import fs from "node:fs";
import path from "node:path";
import { defaultCoderWorktreeDir } from "../../scripts/session/session-policy.mjs";
import { repoPathAbs } from "../../scripts/lib/runtime-paths.mjs";

const wpId = String(process.argv[2] || "").trim();
if (!wpId || !wpId.startsWith("WP-")) {
  console.error("Usage: node install-validator-guard.mjs <WP_ID>");
  process.exit(1);
}

const worktreeDir = repoPathAbs(defaultCoderWorktreeDir(wpId));
if (!fs.existsSync(worktreeDir)) {
  console.error(`[INSTALL_VALIDATOR_GUARD] Worktree not found: ${worktreeDir}`);
  process.exit(1);
}

const claudeDir = path.join(worktreeDir, ".claude");
const settingsPath = path.join(claudeDir, "settings.local.json");

// Load existing settings or start fresh
let settings = {};
if (fs.existsSync(settingsPath)) {
  try {
    settings = JSON.parse(fs.readFileSync(settingsPath, "utf8"));
  } catch {
    console.error(`[INSTALL_VALIDATOR_GUARD] Warning: could not parse existing ${settingsPath}, overwriting.`);
    settings = {};
  }
}

// Ensure hooks.PreToolUse array exists
if (!settings.hooks) settings.hooks = {};
if (!Array.isArray(settings.hooks.PreToolUse)) settings.hooks.PreToolUse = [];

const HOOK_DESCRIPTION = "Blocks validator from editing product code [RGF-105]";
const HOOK_COMMAND = "node .GOV/roles_shared/scripts/hooks/validator-write-guard.mjs";

// Avoid duplicate entries
const alreadyInstalled = settings.hooks.PreToolUse.some(
  (h) => h.command === HOOK_COMMAND
);

if (alreadyInstalled) {
  console.log(`[INSTALL_VALIDATOR_GUARD] Hook already present in ${settingsPath}`);
} else {
  settings.hooks.PreToolUse.push({
    command: HOOK_COMMAND,
    description: HOOK_DESCRIPTION,
  });

  fs.mkdirSync(claudeDir, { recursive: true });
  fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2) + "\n");
  console.log(`[INSTALL_VALIDATOR_GUARD] Installed PreToolUse hook: ${settingsPath}`);
}

console.log(`[INSTALL_VALIDATOR_GUARD] Worktree: ${worktreeDir}`);
console.log(`[INSTALL_VALIDATOR_GUARD] WP: ${wpId}`);
console.log(`[INSTALL_VALIDATOR_GUARD] NOTE: This guard is for VALIDATOR sessions only.`);
console.log(`[INSTALL_VALIDATOR_GUARD]   → Remove or revert settings.local.json before the coder resumes.`);
