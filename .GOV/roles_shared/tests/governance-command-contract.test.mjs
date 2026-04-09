import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const REPO_ROOT = path.resolve(".");
const JUSTFILE_PATH = path.join(REPO_ROOT, "justfile");
const PROTOCOL_ALIGNMENT_CHECK = path.join(
  REPO_ROOT,
  ".GOV",
  "roles_shared",
  "checks",
  "protocol-alignment-check.mjs",
);
const ACTIVE_COMMAND_DOC_PATHS = [
  path.join(REPO_ROOT, ".GOV", "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
  path.join(REPO_ROOT, ".GOV", "roles", "orchestrator", "README.md"),
  path.join(REPO_ROOT, ".GOV", "roles", "coder", "CODER_PROTOCOL.md"),
  path.join(REPO_ROOT, ".GOV", "roles", "validator", "VALIDATOR_PROTOCOL.md"),
  path.join(REPO_ROOT, ".GOV", "roles", "validator", "README.md"),
  path.join(REPO_ROOT, ".GOV", "roles_shared", "docs", "COMMAND_SURFACE_REFERENCE.md"),
  path.join(REPO_ROOT, ".GOV", "roles_shared", "docs", "GOVERNED_WORKFLOW_EXAMPLES.md"),
  path.join(REPO_ROOT, ".GOV", "roles_shared", "docs", "ROLE_SESSION_ORCHESTRATION.md"),
];
const BROAD_RETIRED_COMMAND_DOC_PATHS = [
  ...ACTIVE_COMMAND_DOC_PATHS,
  path.join(REPO_ROOT, ".GOV", "codex", "Handshake_Codex_v1.4.md"),
  path.join(REPO_ROOT, ".GOV", "roles", "coder", "README.md"),
  path.join(REPO_ROOT, ".GOV", "roles_shared", "docs", "QUALITY_GATE.md"),
  path.join(REPO_ROOT, ".GOV", "roles_shared", "docs", "ROLE_WORKFLOW_QUICKREF.md"),
  path.join(REPO_ROOT, ".GOV", "roles_shared", "docs", "START_HERE.md"),
  path.join(REPO_ROOT, ".GOV", "templates", "TASK_PACKET_TEMPLATE.md"),
];

function recipeExists(text, recipeName) {
  const escaped = recipeName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`^${escaped}(?:\\s|:|$)`, "m").test(text);
}

function parseJustRecipes(text) {
  const recipes = new Set();
  const lines = String(text || "").split(/\r?\n/);
  for (const line of lines) {
    if (!line || /^\s/.test(line) || /^\s*#/.test(line)) continue;
    const match = line.match(/^([A-Za-z0-9][A-Za-z0-9-]*)\b/);
    if (!match) continue;
    recipes.add(match[1]);
  }
  return recipes;
}

function extractJustCommands(text) {
  const matches = String(text || "").matchAll(/`just\s+([a-z0-9]+(?:-[a-z0-9]+)*)(?=\s|`)/g);
  return [...new Set([...matches].map((match) => match[1]))].sort();
}

test("governance command contract keeps the orchestrator ACP/session-control recipes exposed", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");
  const requiredRecipes = [
    "ensure-wp-communications",
    "launch-coder-session",
    "launch-wp-validator-session",
    "launch-integration-validator-session",
    "orchestrator-steer-next",
    "manual-relay-next",
    "manual-relay-dispatch",
    "start-coder-session",
    "start-wp-validator-session",
    "start-integration-validator-session",
    "steer-coder-session",
    "steer-wp-validator-session",
    "steer-integration-validator-session",
    "cancel-coder-session",
    "cancel-wp-validator-session",
    "cancel-integration-validator-session",
    "close-coder-session",
    "close-wp-validator-session",
    "close-integration-validator-session",
    "session-start",
    "session-send",
    "session-cancel",
    "session-close",
    "handshake-acp-broker-status",
    "handshake-acp-broker-stop",
    "phase-check",
    "wp-validator-query",
    "wp-review-request",
    "wp-validator-response",
    "wp-review-response",
    "operator-viewport",
    "operator-viewport-admin",
    "operator-monitor",
    "operator-admin",
  ];

  for (const recipeName of requiredRecipes) {
    assert.equal(recipeExists(justfile, recipeName), true, `Missing just recipe: ${recipeName}`);
  }

  for (const retiredRecipeName of [
    "phase-check-startup",
    "phase-check-handoff",
    "phase-check-verdict",
    "phase-check-closeout",
    "gate-check",
    "pre-work",
    "post-work",
    "validator-packet-complete",
    "validator-handoff-check",
    "integration-validator-closeout-check",
    "integration-validator-closeout-sync",
  ]) {
    assert.equal(recipeExists(justfile, retiredRecipeName), false, `Retired just recipe still exposed: ${retiredRecipeName}`);
  }
});

test("governance command contract blocks active command docs from drifting away from the live justfile", () => {
  const recipes = parseJustRecipes(fs.readFileSync(JUSTFILE_PATH, "utf8"));
  const missing = [];

  for (const docPath of ACTIVE_COMMAND_DOC_PATHS) {
    const commands = extractJustCommands(fs.readFileSync(docPath, "utf8"));
    for (const command of commands) {
      if (!recipes.has(command)) {
        missing.push(`${path.relative(REPO_ROOT, docPath)} -> just ${command}`);
      }
    }
  }

  assert.deepEqual(missing, []);
});

test("governance command contract keeps retired named phase-check wrappers out of active docs", () => {
  const staleMatches = [];
  const staleRegex = /phase-check-startup|phase-check-handoff|phase-check-verdict|phase-check-closeout|just pre-work\b|just gate-check\b|just post-work\b|just integration-validator-closeout-sync\b/;

  for (const docPath of BROAD_RETIRED_COMMAND_DOC_PATHS) {
    const content = fs.readFileSync(docPath, "utf8");
    if (staleRegex.test(content)) {
      staleMatches.push(path.relative(REPO_ROOT, docPath));
    }
  }

  assert.deepEqual(staleMatches, []);
});

test("governance command contract preserves quoted GOV_ROOT-backed node invocations in the justfile", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");

  assert.equal(
    /(^\s*@?node )\{\{GOV_ROOT\}\}\//m.test(justfile),
    false,
    "found unquoted node {{GOV_ROOT}}/ invocation",
  );
  assert.equal(
    /;\s*node \{\{GOV_ROOT\}\}\//m.test(justfile),
    false,
    "found unquoted inline node {{GOV_ROOT}}/ invocation",
  );
  assert.match(justfile, /phase-check phase wp-id role="" session="" \*args:/);
});

test("governance command contract keeps protocol-alignment-check green for quoted session-control recipes", () => {
  const result = spawnSync(process.execPath, [PROTOCOL_ALIGNMENT_CHECK], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_ACTIVE_REPO_ROOT: REPO_ROOT,
    },
  });

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /protocol-alignment-check ok/);
});

test("protocol-alignment-check follows HANDSHAKE_GOV_ROOT instead of HANDSHAKE_ACTIVE_REPO_ROOT", () => {
  const activeRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "protocol-alignment-active-"));

  try {
    const result = spawnSync(process.execPath, [PROTOCOL_ALIGNMENT_CHECK], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      env: {
        ...process.env,
        HANDSHAKE_ACTIVE_REPO_ROOT: activeRepoRoot,
        HANDSHAKE_GOV_ROOT: path.join(REPO_ROOT, ".GOV"),
      },
    });

    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /protocol-alignment-check ok/);
  } finally {
    fs.rmSync(activeRepoRoot, { recursive: true, force: true });
  }
});
