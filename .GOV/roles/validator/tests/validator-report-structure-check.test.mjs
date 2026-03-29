import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

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
    [path.join(".GOV", "roles", "validator", "checks", "validator-report-structure-check.mjs")],
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
    [path.join(".GOV", "roles", "validator", "checks", "validator-report-structure-check.mjs")],
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
