import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const checkerPath = path.resolve(testDir, "..", "checks", "validator-report-structure-check.mjs");

function writeFile(targetPath, content) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, content, "utf8");
}

test("validator-report-structure-check scans folder packets instead of only flat packet files", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "validator-report-structure-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const packetPath = path.join(govRoot, "task_packets", "WP-TEST-FOLDER-v1", "packet.md");

  writeFile(
    packetPath,
    [
      "# WP-TEST-FOLDER-v1",
      "",
      "- **Status:** Done",
      "- PACKET_FORMAT_VERSION: 2026-03-22",
      "- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3",
      "",
      "## VALIDATION_REPORTS",
      "",
    ].join("\n"),
  );

  const result = spawnSync(
    process.execPath,
    [checkerPath],
    {
      cwd: process.cwd(),
      encoding: "utf8",
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
      },
    },
  );

  assert.equal(result.status, 1);
  assert.match(result.stderr, /WP-TEST-FOLDER-v1\/packet\.md:/i);
  assert.match(result.stderr, /VALIDATION_REPORTS missing\/empty for closed packet/i);
});

test("validator-report-structure-check accepts Validated (ABANDONED) packets with matching disposition", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "validator-report-structure-abandoned-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const packetPath = path.join(govRoot, "task_packets", "WP-TEST-ABANDONED-v1", "packet.md");

  writeFile(
    packetPath,
    [
      "# WP-TEST-ABANDONED-v1",
      "",
      "- **Status:** Validated (ABANDONED)",
      "- PACKET_FORMAT_VERSION: 2026-03-22",
      "- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V2",
      "",
      "## VALIDATION_REPORTS",
      "Verdict: ABANDONED",
      "VALIDATION_CONTEXT: CONTEXT_MISMATCH",
      "GOVERNANCE_VERDICT: BLOCKED",
      "TEST_VERDICT: NOT_RUN",
      "CODE_REVIEW_VERDICT: NOT_RUN",
      "HEURISTIC_REVIEW_VERDICT: NOT_RUN",
      "SPEC_ALIGNMENT_VERDICT: BLOCKED",
      "ENVIRONMENT_VERDICT: NOT_RUN",
      "DISPOSITION: ABANDONED",
      "LEGAL_VERDICT: PENDING",
      "SPEC_CONFIDENCE: NONE",
      "WORKFLOW_VALIDITY: BLOCKED",
      "SCOPE_VALIDITY: PARTIAL",
      "PROOF_COMPLETENESS: NOT_PROVEN",
      "INTEGRATION_READINESS: NOT_READY",
      "DOMAIN_GOAL_COMPLETION: INCOMPLETE",
      "CLAUSES_REVIEWED:",
      "- NONE",
      "NOT_PROVEN:",
      "- packet intentionally abandoned before governed proof completion",
      "MAIN_BODY_GAPS:",
      "- NONE",
      "QUALITY_RISKS:",
      "- NONE",
      "",
    ].join("\n"),
  );

  const result = spawnSync(
    process.execPath,
    [checkerPath],
    {
      cwd: process.cwd(),
      encoding: "utf8",
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
      },
    },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
});

test("validator-report-structure-check rejects PASS packets whose NEGATIVE_PROOF is governance-only", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "validator-report-structure-negative-proof-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const packetPath = path.join(govRoot, "task_packets", "WP-TEST-NEGATIVE-PROOF-v1", "packet.md");

  writeFile(
    packetPath,
    [
      "# WP-TEST-NEGATIVE-PROOF-v1",
      "",
      "- **Status:** Validated (PASS)",
      "- PACKET_FORMAT_VERSION: 2026-03-26",
      "- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3",
      "- RISK_TIER: HIGH",
      "",
      "## VALIDATION_REPORTS",
      "Verdict: PASS",
      "VALIDATION_CONTEXT: OK",
      "GOVERNANCE_VERDICT: PASS",
      "TEST_VERDICT: PASS",
      "CODE_REVIEW_VERDICT: PASS",
      "HEURISTIC_REVIEW_VERDICT: PASS",
      "SPEC_ALIGNMENT_VERDICT: PASS",
      "ENVIRONMENT_VERDICT: PASS",
      "DISPOSITION: NONE",
      "LEGAL_VERDICT: PASS",
      "SPEC_CONFIDENCE: HIGH",
      "VALIDATOR_RISK_TIER: HIGH",
      "WORKFLOW_VALIDITY: VALID",
      "SCOPE_VALIDITY: IN_SCOPE",
      "PROOF_COMPLETENESS: PROVEN",
      "INTEGRATION_READINESS: READY",
      "DOMAIN_GOAL_COMPLETION: COMPLETE",
      "CLAUSES_REVIEWED:",
      "- `[X]` -> `src/demo.rs:10`",
      "NOT_PROVEN:",
      "- NONE",
      "MAIN_BODY_GAPS:",
      "- NONE",
      "QUALITY_RISKS:",
      "- NONE",
      "DIFF_ATTACK_SURFACES:",
      "- `src/demo.rs:10`",
      "INDEPENDENT_CHECKS_RUN:",
      "- `src/demo.rs:10`",
      "- `src/demo.rs:11`",
      "COUNTERFACTUAL_CHECKS:",
      "- `demo::guard()`",
      "- `demo::fallback()`",
      "INDEPENDENT_FINDINGS:",
      "- NONE",
      "RESIDUAL_UNCERTAINTY:",
      "- product-level parser ambiguity still exists",
      "BOUNDARY_PROBES:",
      "- `src/demo.rs:10`",
      "NEGATIVE_PATH_CHECKS:",
      "- `src/demo.rs:11`",
      "SPEC_CLAUSE_MAP:",
      "- `[X]` -> `src/demo.rs:10`",
      "NEGATIVE_PROOF:",
      "- Repo governance still blocks `integration-validator-closeout-check` inside `.GOV/roles/validator/VALIDATOR_PROTOCOL.md` and is outside the signed product scope.",
      "",
    ].join("\n"),
  );

  const result = spawnSync(
    process.execPath,
    [checkerPath],
    {
      cwd: process.cwd(),
      encoding: "utf8",
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
      },
    },
  );

  assert.equal(result.status, 1);
  assert.match(result.stderr, /NEGATIVE_PROOF entries to stay inside signed product scope/i);
});
