import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { validateDiscoveryHbrPointers } from "./discovery-hbr-pointers.mjs";

const GOV_CHECK_SCRIPT = fileURLToPath(new URL("./gov-check.mjs", import.meta.url));

function withFixture(fn) {
  const root = mkdtempSync(path.join(tmpdir(), "discovery-hbr-pointers-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function writeRepoFile(repoRoot, relativePath, content) {
  const filePath = path.join(repoRoot, relativePath);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, `${dedent(content).trim()}\n`, "utf8");
}

function dedent(content) {
  const lines = String(content).replace(/\r\n/g, "\n").split("\n");
  const nonEmptyLines = lines.filter((line) => line.trim() !== "");
  const indent = nonEmptyLines.reduce((minimum, line) => {
    const width = line.match(/^\s*/)?.[0].length ?? 0;
    return Math.min(minimum, width);
  }, Number.POSITIVE_INFINITY);
  if (!Number.isFinite(indent) || indent === 0) return String(content);
  return lines.map((line) => line.slice(indent)).join("\n");
}

function writeValidDiscovery(repoRoot) {
  writeRepoFile(repoRoot, ".GOV/roles_shared/docs/START_HERE.md", `
    # Start Here
    ## Build Rules (HBR)
    .GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json
    .GOV/spec/master-spec-v02.186/spec-modules/05-security-and-observability.md#5.6
    CX-131
    CX-503B1
    packet.acceptance_matrix.hbr
    just hbr-matrix-check
  `);
  writeRepoFile(repoRoot, ".GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md", `
    # Role Session
    ## HBR Handoff Gate
    HandoffGate (MT-004) MUST PASS
    refinement->coder
    coder->WP_VALIDATOR
    WP_VALIDATOR->INTEGRATION_VALIDATOR
    INTEGRATION_VALIDATOR->ORCHESTRATOR
    .GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json
    packet.acceptance_matrix.hbr
    .GOV/roles/coder/CODER_PROTOCOL.md
    .GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md
    .GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md
    .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md
    .GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md
    .GOV/roles/validator/VALIDATOR_PROTOCOL.md
    .GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md
  `);
  writeRepoFile(repoRoot, ".GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md", `
    # Playbook
    just hbr-matrix-check before any Coder handoff
    just hbr-visual-smoke
    just hbr-swarm-n8
    just hbr-inspector-smoke before Integration Validator closeout
  `);
  writeRepoFile(repoRoot, "justfile", `
hbr-matrix-check *FLAGS="":
  node check
hbr-visual-smoke *FLAGS="":
  node check
hbr-swarm-n8 *FLAGS="":
  node check
hbr-inspector-smoke *FLAGS="":
  node check
`);
}

function runGovCheckOnly(repoRoot) {
  return spawnSync(process.execPath, [GOV_CHECK_SCRIPT, "--json"], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
      HANDSHAKE_GOV_ROOT: path.join(repoRoot, ".GOV"),
      HANDSHAKE_GOV_RUNTIME_ROOT: path.join(repoRoot, ".runtime"),
      HANDSHAKE_GOV_CHECK_TEST_MODE: "1",
      HANDSHAKE_GOV_CHECK_ONLY: "discovery-hbr-pointers",
    },
  });
}

function runGovCheckOnlyWithEnv(cwd, env) {
  return spawnSync(process.execPath, [GOV_CHECK_SCRIPT, "--json"], {
    cwd,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_GOV_CHECK_TEST_MODE: "1",
      HANDSHAKE_GOV_CHECK_ONLY: "discovery-hbr-pointers",
      ...env,
    },
  });
}

test("passes when discovery surfaces expose HBR pointers", () => withFixture((repoRoot) => {
  writeValidDiscovery(repoRoot);

  assert.deepEqual(validateDiscoveryHbrPointers(repoRoot), []);
}));

test("fails clearly when discovery pointers or recipes are absent", () => withFixture((repoRoot) => {
  writeValidDiscovery(repoRoot);
  writeRepoFile(repoRoot, ".GOV/roles_shared/docs/START_HERE.md", "# Start Here");
  writeRepoFile(repoRoot, "justfile", "hbr-visual-smoke:\n  node check");

  const failures = validateDiscoveryHbrPointers(repoRoot);

  assert(failures.some((failure) => failure.includes("HANDSHAKE_BUILD_RULES.json")));
  assert(failures.some((failure) => failure.includes("hbr-matrix-check")));
}));

test("discovery HBR checker is wired into gov-check bundle runtime path", () => withFixture((repoRoot) => {
  writeValidDiscovery(repoRoot);

  const result = runGovCheckOnly(repoRoot);

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stdout, /discovery-hbr-pointers ok/);
  assert.match(result.stdout, /gov-check ok/);
}));

test("gov-check discovery HBR checker resolves docs from HANDSHAKE_GOV_ROOT in split-root mode", () => withFixture((root) => {
  const activeProductRoot = path.join(root, "product-worktree");
  const govWorkspaceRoot = path.join(root, "gov-worktree");
  mkdirSync(activeProductRoot, { recursive: true });
  writeValidDiscovery(govWorkspaceRoot);

  const result = runGovCheckOnlyWithEnv(activeProductRoot, {
    HANDSHAKE_ACTIVE_REPO_ROOT: activeProductRoot,
    HANDSHAKE_GOV_ROOT: path.join(govWorkspaceRoot, ".GOV"),
    HANDSHAKE_GOV_RUNTIME_ROOT: path.join(root, ".runtime"),
  });

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stdout, /discovery-hbr-pointers ok/);
  assert.match(result.stdout, /gov-check ok/);
}));

test("unreadable discovery docs report the unreadable file without derivative missing-needle noise", () => withFixture((repoRoot) => {
  writeValidDiscovery(repoRoot);
  rmSync(path.join(repoRoot, ".GOV/roles_shared/docs/START_HERE.md"), { force: true });

  const failures = validateDiscoveryHbrPointers(repoRoot);
  const startHereFailures = failures.filter((failure) => failure.includes(".GOV/roles_shared/docs/START_HERE.md"));

  assert.equal(startHereFailures.length, 1, failures.join("\n"));
  assert.match(startHereFailures[0], /unreadable/);
}));
