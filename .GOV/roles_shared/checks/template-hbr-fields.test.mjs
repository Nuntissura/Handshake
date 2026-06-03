import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const CHECK_SCRIPT = fileURLToPath(new URL("./template-hbr-fields.mjs", import.meta.url));
const GOV_CHECK_SCRIPT = fileURLToPath(new URL("./gov-check.mjs", import.meta.url));

function withFixture(fn) {
  const root = mkdtempSync(path.join(tmpdir(), "template-hbr-fields-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function writeRepoFile(repoRoot, relativePath, content) {
  const filePath = path.join(repoRoot, relativePath);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, `${content.trim()}\n`, "utf8");
}

function writeJson(repoRoot, relativePath, value) {
  writeRepoFile(repoRoot, relativePath, JSON.stringify(value, null, 2));
}

function validTaskPacketMarkdown() {
  return `
    ## PACKET_ACCEPTANCE_MATRIX
    - hbr.tags_declared: []
    - hbr.not_applicable_overrides: []
    - acceptance_matrix.schema_version: 1
    - acceptance_matrix.hbr: []
    - acceptance_matrix.hbr_not_applicable: []
  `;
}

function validRefinementMarkdown() {
  return `
    ### HBR_PILLAR_REVIEW
    - hbr_pillar_review:
      - INT: applicable: <YES|NO|UNKNOWN> | evidence_path: <path or NONE>
      - SWARM: applicable: <YES|NO|UNKNOWN> | evidence_path: <path or NONE>
      - VIS: applicable: <YES|NO|UNKNOWN> | evidence_path: <path or NONE>
      - QUIET: applicable: <YES|NO|UNKNOWN> | evidence_path: <path or NONE>
      - MAN: applicable: <YES|NO|UNKNOWN> | evidence_path: <path or NONE>
  `;
}

function validWorkPacketContract() {
  return {
    hbr: {
      tags_declared: [],
      not_applicable_overrides: [],
    },
    acceptance_matrix: {
      schema_version: 1,
      hbr: [],
      hbr_not_applicable: [],
    },
  };
}

function validPillarReview() {
  return Object.fromEntries(["INT", "SWARM", "VIS", "QUIET", "MAN"].map((pillar) => [
    pillar,
    {
      applicable: null,
      evidence_path: null,
    },
  ]));
}

function validRefinementContract() {
  return {
    refinement: {
      hbr_pillar_review: validPillarReview(),
      microtask_plan: [],
      microtask_plan_item_defaults: {
        hbr_obligations: [],
      },
    },
  };
}

function validMicrotaskContract() {
  return {
    hbr_obligations: [],
    scope: {
      summary: "{{MT_SUMMARY}}",
    },
  };
}

function validGeneratorSource() {
  return `
    import {
      buildDefaultHbrAcceptanceMatrix,
      buildDefaultHbrContext,
      buildDefaultHbrObligations,
    } from "./defaults.mjs";

    const packet = {
      hbr: buildDefaultHbrContext(),
      acceptance_matrix: buildDefaultHbrAcceptanceMatrix(),
    };
    const mt = {
      hbr_obligations: buildDefaultHbrObligations(),
    };
    void packet;
    void mt;
  `;
}

function writeValidTemplates(repoRoot) {
  writeRepoFile(repoRoot, ".GOV/templates/TASK_PACKET_TEMPLATE.md", validTaskPacketMarkdown());
  writeJson(repoRoot, ".GOV/templates/WORK_PACKET_CONTRACT_TEMPLATE.json", validWorkPacketContract());
  writeRepoFile(repoRoot, ".GOV/templates/REFINEMENT_TEMPLATE.md", validRefinementMarkdown());
  writeJson(repoRoot, ".GOV/templates/REFINEMENT_CONTRACT_TEMPLATE.json", validRefinementContract());
  writeJson(repoRoot, ".GOV/templates/MICRO_TASK_CONTRACT_TEMPLATE.json", validMicrotaskContract());
  writeRepoFile(repoRoot, ".GOV/roles/orchestrator/scripts/create-task-packet.mjs", validGeneratorSource());
  writeRepoFile(repoRoot, ".GOV/roles_shared/scripts/wp/atelier-lens-consolidation-packet-generator.mjs", validGeneratorSource());
}

function runCheck(repoRoot) {
  return spawnSync(process.execPath, [CHECK_SCRIPT, "--repo-root", repoRoot], {
    cwd: repoRoot,
    encoding: "utf8",
  });
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
      HANDSHAKE_GOV_CHECK_ONLY: "template-hbr-fields",
    },
  });
}

test("passes when all template HBR defaults are present", () => withFixture((repoRoot) => {
  writeValidTemplates(repoRoot);

  const result = runCheck(repoRoot);

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /template-hbr-fields ok/);
  assert.match(result.stdout, /MICRO_TASK_CONTRACT_TEMPLATE\.json/);
}));

test("fails clearly when required template HBR fields are absent", () => withFixture((repoRoot) => {
  writeValidTemplates(repoRoot);
  writeRepoFile(repoRoot, ".GOV/templates/TASK_PACKET_TEMPLATE.md", "## PACKET_ACCEPTANCE_MATRIX");
  writeJson(repoRoot, ".GOV/templates/MICRO_TASK_CONTRACT_TEMPLATE.json", { scope: {} });

  const result = runCheck(repoRoot);
  const report = JSON.parse(result.stderr);

  assert.equal(result.status, 1);
  assert.equal(report.check, "template-hbr-fields");
  assert.equal(report.verdict, "FAIL");
  assert(report.failures.some((failure) => failure.includes("hbr.tags_declared")));
  assert(report.failures.some((failure) => failure.includes("hbr_obligations")));
}));

test("fails when HBR defaults are pre-populated instead of minimal", () => withFixture((repoRoot) => {
  writeValidTemplates(repoRoot);
  writeJson(repoRoot, ".GOV/templates/WORK_PACKET_CONTRACT_TEMPLATE.json", {
    hbr: {
      tags_declared: ["HBR-INT"],
      not_applicable_overrides: [{ hbr_id: "HBR-VIS-001", reason: "pre-filled" }],
    },
    acceptance_matrix: {
      schema_version: 1,
      hbr: [{ hbr_id: "HBR-INT-001" }],
      hbr_not_applicable: [{ hbr_id: "HBR-VIS-001", reason: "pre-filled" }],
    },
  });
  writeJson(repoRoot, ".GOV/templates/MICRO_TASK_CONTRACT_TEMPLATE.json", {
    hbr_obligations: ["HBR-INT-001"],
  });

  const result = runCheck(repoRoot);
  const report = JSON.parse(result.stderr);

  assert.equal(result.status, 1);
  assert(report.failures.some((failure) => failure.includes("hbr.tags_declared")));
  assert(report.failures.some((failure) => failure.includes("acceptance_matrix.hbr")));
  assert(report.failures.some((failure) => failure.includes("hbr_obligations")));
}));

test("template HBR checker is wired into gov-check bundle runtime path", () => withFixture((repoRoot) => {
  writeValidTemplates(repoRoot);

  const result = runGovCheckOnly(repoRoot);

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stdout, /template-hbr-fields ok/);
  assert.match(result.stdout, /gov-check ok/);
}));
