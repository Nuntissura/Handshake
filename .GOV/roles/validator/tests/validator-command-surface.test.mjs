import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const JUSTFILE_PATH = path.resolve("justfile");
const COMMAND_SURFACE_REFERENCE_PATH = path.resolve(".GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md");
const VALIDATOR_PROTOCOL_PATH = path.resolve(".GOV/roles/validator/VALIDATOR_PROTOCOL.md");

function recipeExists(text, recipeName) {
  const escaped = recipeName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`^${escaped}(?:\\s|:|$)`, "m").test(text);
}

test("justfile exposes the live validator command surface referenced by validator docs and helpers", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");
  const requiredRecipes = [
    "validator-handoff-check",
    "integration-validator-closeout-check",
    "integration-validator-context-brief",
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

test("critical integration-validator helper commands stay aligned across docs, protocol, and justfile", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");
  const commandSurface = fs.readFileSync(COMMAND_SURFACE_REFERENCE_PATH, "utf8");
  const validatorProtocol = fs.readFileSync(VALIDATOR_PROTOCOL_PATH, "utf8");

  assert.match(commandSurface, /`just integration-validator-context-brief WP-\{ID\} \[--json\]`/);
  assert.match(commandSurface, /`just integration-validator-closeout-check WP-\{ID\}`/);
  assert.match(commandSurface, /`just wp-token-usage WP-\{ID\}`/);
  assert.match(commandSurface, /`just external-validator-brief WP-\{ID\} \[--json\]`/);
  assert.match(validatorProtocol, /just integration-validator-context-brief WP-\{ID\}/);
  assert.equal(recipeExists(justfile, "integration-validator-context-brief"), true);
  assert.equal(recipeExists(justfile, "integration-validator-closeout-check"), true);
  assert.equal(recipeExists(justfile, "wp-token-usage"), true);
});
