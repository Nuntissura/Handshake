import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const JUSTFILE_PATH = path.resolve("justfile");

function recipeExists(text, recipeName) {
  const escaped = recipeName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`^${escaped}(?:\\s|:|$)`, "m").test(text);
}

test("justfile exposes the live validator command surface referenced by validator docs and helpers", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");
  const requiredRecipes = [
    "validator-handoff-check",
    "integration-validator-closeout-check",
    "external-validator-brief",
    "gate-check",
    "spec-eof-appendices-check",
    "task-board-set",
    "start-wp-validator-session",
    "start-integration-validator-session",
    "steer-wp-validator-session",
    "steer-integration-validator-session",
    "cancel-wp-validator-session",
    "cancel-integration-validator-session",
    "validator-gate-append",
    "validator-gate-commit",
    "validator-gate-present",
    "validator-gate-acknowledge",
    "validator-gate-status",
    "validator-gate-reset",
    "validator-governance-snapshot",
    "validator-report-structure-check",
    "validator-phase-gate",
    "validator-error-codes",
    "validator-coverage-gaps",
    "validator-traceability",
    "validator-hygiene-full",
    "session-control-runtime-check",
    "handshake-acp-broker-status",
    "sync-all-role-worktrees",
    "reseed-permanent-worktree-from-main",
    "generate-worktree-cleanup-script",
    "close-wp-branch",
    "wp-heartbeat",
    "wp-spec-gap",
    "wp-spec-confirmation",
    "wp-validator-response",
    "wp-review-response",
  ];

  for (const recipeName of requiredRecipes) {
    assert.equal(recipeExists(justfile, recipeName), true, `Missing just recipe: ${recipeName}`);
  }
});
