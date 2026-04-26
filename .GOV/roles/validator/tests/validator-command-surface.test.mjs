import assert from "node:assert/strict";
import { execSync } from "node:child_process";
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

function readCanonicalMainJustfile() {
  return execSync("git show main:justfile", { encoding: "utf8" });
}

test("justfile exposes the live validator command surface referenced by validator docs and helpers", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");
  const requiredRecipes = [
    "phase-check",
    "integration-validator-context-brief",
    "external-validator-brief",
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
  assert.match(commandSurface, /`just phase-check <STARTUP\|HANDOFF\|VERDICT\|CLOSEOUT> WP-\{ID\} \[ROLE\] \[session\]`/);
  assert.match(commandSurface, /phase-check CLOSEOUT WP-\{ID\} \[ROLE\] \[session\] --sync-mode <MERGE_PENDING\|CONTAINED_IN_MAIN\|FAIL\|OUTDATED_ONLY\|ABANDONED> --context/);
  assert.match(commandSurface, /`just wp-token-usage WP-\{ID\}`/);
  assert.match(commandSurface, /`just external-validator-brief WP-\{ID\} \[--json\]`/);
  assert.match(validatorProtocol, /just integration-validator-context-brief WP-\{ID\}/);
  assert.match(validatorProtocol, /just phase-check CLOSEOUT WP-\{ID\}/);
  assert.match(validatorProtocol, /--sync-mode MERGE_PENDING --context/);
  assert.equal(recipeExists(justfile, "phase-check"), true);
  assert.equal(recipeExists(justfile, "integration-validator-context-brief"), true);
  assert.equal(recipeExists(justfile, "integration-validator-closeout-sync"), false);
  assert.equal(recipeExists(justfile, "integration-validator-closeout-check"), false);
  assert.equal(recipeExists(justfile, "validator-handoff-check"), false);
  assert.equal(recipeExists(justfile, "validator-packet-complete"), false);
  assert.equal(recipeExists(justfile, "wp-token-usage"), true);
});

test("windows-sensitive validator wrappers route variadic flags through node-argv-proxy", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");

  assert.ok(
    justfile.includes('node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles/validator/scripts/validator-next.mjs" --role {{role}} {{wp-id}} --raw-flags "{{FLAGS}}"'),
    "validator-next must route variadic flags through node-argv-proxy",
  );
  assert.ok(
    justfile.includes('node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs" {{wp-id}} --raw-flags "{{args}}"'),
    "integration-validator-context-brief must route variadic flags through node-argv-proxy",
  );
  assert.ok(
    justfile.includes('node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/checks/phase-check.mjs" {{phase}} {{wp-id}} {{role}} "{{session}}" --raw-flags "{{args}}"'),
    "phase-check must route variadic flags through node-argv-proxy",
  );
});

test("canonical main justfile preserves the governed WP command surface used by active worktrees", () => {
  const mainJustfile = readCanonicalMainJustfile();
  const requiredMainRecipes = [
    "phase-check",
    "wp-invalidity-flag",
    "wp-operator-rule-restatement",
    "wp-review-exchange",
    "wp-spec-gap",
    "wp-spec-confirmation",
    "wp-communication-health-check",
    "check-notifications",
    "ack-notifications",
  ];

  for (const recipeName of requiredMainRecipes) {
    assert.equal(recipeExists(mainJustfile, recipeName), true, `main justfile is missing canonical recipe: ${recipeName}`);
  }
});
