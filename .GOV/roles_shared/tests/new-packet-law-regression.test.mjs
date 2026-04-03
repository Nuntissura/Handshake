import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  formatDataContractDecisionSection,
  formatDataContractMonitoringSection,
} from "../scripts/lib/data-contract-lib.mjs";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const claimCheckPath = path.resolve(testDir, "..", "checks", "task-packet-claim-check.mjs");
const reportCheckPath = path.resolve(testDir, "..", "..", "roles", "validator", "checks", "validator-report-structure-check.mjs");

function writeFile(targetPath, content) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, content, "utf8");
}

function runNode(scriptPath, govRoot) {
  return spawnSync(process.execPath, [scriptPath], {
    cwd: process.cwd(),
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_GOV_ROOT: govRoot,
    },
  });
}

test("2026-04-01 packet law passes claim and closure checks when data-contract activation is explicit", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "packet-law-regression-pass-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const readyPacketPath = path.join(govRoot, "task_packets", "WP-TEST-READY-v1", "packet.md");
  const closedPacketPath = path.join(govRoot, "task_packets", "WP-TEST-CLOSED-v1", "packet.md");

  writeFile(
    readyPacketPath,
    [
      "# WP-TEST-READY-v1",
      "",
      "- **Status:** Ready for Dev",
      "- PACKET_FORMAT_VERSION: 2026-04-01",
      "- TOUCHED_FILE_BUDGET: 1",
      "- BROAD_TOOL_ALLOWLIST: NONE",
      "- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1",
      "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
      "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A",
      "- PACKET_WIDENING_DECISION: NONE",
      "- PACKET_WIDENING_EVIDENCE: N/A",
      "",
      "## IN_SCOPE_PATHS",
      "- src/backend/handshake_core/src/locus/types.rs",
      "",
      "## OUT_OF_SCOPE",
      "- src/frontend/app.tsx",
      "",
      formatDataContractDecisionSection({
        decision: "ACTIVE_REQUIRED",
        reason: "Current packet scope includes a concrete backend type surface that persists or emits structured data.",
        evidence: ["IN_SCOPE_PATH: src/backend/handshake_core/src/locus/types.rs (backend data surface)"],
      }).trim(),
      "",
      formatDataContractMonitoringSection({
        profile: "LLM_FIRST_DATA_V1",
        inScopePaths: ["src/backend/handshake_core/src/locus/types.rs"],
      }).trim(),
      "",
    ].join("\n"),
  );

  writeFile(
    closedPacketPath,
    [
      "# WP-TEST-CLOSED-v1",
      "",
      "- **Status:** Validated (PASS)",
      "- PACKET_FORMAT_VERSION: 2026-04-01",
      "- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3",
      "- RISK_TIER: MEDIUM",
      "- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1",
      "",
      "## IN_SCOPE_PATHS",
      "- src/backend/handshake_core/src/locus/types.rs",
      "",
      "## OUT_OF_SCOPE",
      "- src/frontend/app.tsx",
      "",
      formatDataContractDecisionSection({
        decision: "ACTIVE_REQUIRED",
        reason: "Current packet scope includes a concrete backend type surface that persists or emits structured data.",
        evidence: ["IN_SCOPE_PATH: src/backend/handshake_core/src/locus/types.rs (backend data surface)"],
      }).trim(),
      "",
      formatDataContractMonitoringSection({
        profile: "LLM_FIRST_DATA_V1",
        inScopePaths: ["src/backend/handshake_core/src/locus/types.rs"],
      }).trim(),
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
      "VALIDATOR_RISK_TIER: MEDIUM",
      "WORKFLOW_VALIDITY: VALID",
      "SCOPE_VALIDITY: IN_SCOPE",
      "PROOF_COMPLETENESS: PROVEN",
      "INTEGRATION_READINESS: READY",
      "DOMAIN_GOAL_COMPLETION: COMPLETE",
      "CLAUSES_REVIEWED:",
      "- `[X]` -> `src/backend/handshake_core/src/locus/types.rs:10`",
      "NOT_PROVEN:",
      "- NONE",
      "MAIN_BODY_GAPS:",
      "- NONE",
      "QUALITY_RISKS:",
      "- NONE",
      "ANTI_VIBE_FINDINGS:",
      "- NONE",
      "SIGNED_SCOPE_DEBT:",
      "- NONE",
      "DIFF_ATTACK_SURFACES:",
      "- `src/backend/handshake_core/src/locus/types.rs:10`",
      "INDEPENDENT_CHECKS_RUN:",
      "- `src/backend/handshake_core/src/locus/types.rs:10`",
      "COUNTERFACTUAL_CHECKS:",
      "- `locus::serialize_profile()`",
      "BOUNDARY_PROBES:",
      "- `src/backend/handshake_core/src/locus/types.rs:12`",
      "NEGATIVE_PATH_CHECKS:",
      "- `src/backend/handshake_core/src/locus/types.rs:14`",
      "INDEPENDENT_FINDINGS:",
      "- validator confirmed the emitted field names remain explicit",
      "RESIDUAL_UNCERTAINTY:",
      "- residual portability confidence is bounded to the reviewed type surface",
      "SPEC_CLAUSE_MAP:",
      "- `[X]` -> `src/backend/handshake_core/src/locus/types.rs:10`",
      "NEGATIVE_PROOF:",
      "- `src/backend/handshake_core/src/locus/types.rs:20` does not introduce opaque payload semantics",
      "DATA_CONTRACT_PROOF:",
      "- `src/backend/handshake_core/src/locus/types.rs:10` preserves explicit ids and fielded output",
      "DATA_CONTRACT_GAPS:",
      "- NONE",
      "",
    ].join("\n"),
  );

  const claimResult = runNode(claimCheckPath, govRoot);
  assert.equal(claimResult.status, 0, claimResult.stderr || claimResult.stdout);

  const reportResult = runNode(reportCheckPath, govRoot);
  assert.equal(reportResult.status, 0, reportResult.stderr || reportResult.stdout);
});

test("orchestrator-managed ready packets with an assigned coder fail fast when claim fields are still unclaimed", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "packet-law-regression-governed-ready-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const packetPath = path.join(govRoot, "task_packets", "WP-TEST-GOVERNED-READY-v1", "packet.md");

  writeFile(
    packetPath,
    [
      "# WP-TEST-GOVERNED-READY-v1",
      "",
      "- **Status:** Ready for Dev",
      "- PACKET_FORMAT_VERSION: 2026-04-01",
      "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
      "- EXECUTION_OWNER: CODER_A",
      "- CODER_MODEL: <unclaimed>",
      "- CODER_REASONING_STRENGTH: <unclaimed>",
      "- TOUCHED_FILE_BUDGET: 1",
      "- BROAD_TOOL_ALLOWLIST: NONE",
      "- DATA_CONTRACT_PROFILE: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
      "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A",
      "- PACKET_WIDENING_DECISION: NONE",
      "- PACKET_WIDENING_EVIDENCE: N/A",
      "",
      "## IN_SCOPE_PATHS",
      "- .GOV/roles_shared/checks/task-packet-claim-check.mjs",
      "",
      "## OUT_OF_SCOPE",
      "- src/backend/handshake_core/src/locus/types.rs",
      "",
      formatDataContractDecisionSection({
        decision: "WAIVED_NOT_DATA_BEARING",
        reason: "The packet scope is governance-only and does not introduce governed product data surfaces.",
        evidence: ["IN_SCOPE_PATH: .GOV/roles_shared/checks/task-packet-claim-check.mjs"],
      }).trim(),
      "",
      formatDataContractMonitoringSection({
        profile: "NONE",
        inScopePaths: [],
      }).trim(),
      "",
    ].join("\n"),
  );

  const claimResult = runNode(claimCheckPath, govRoot);
  assert.equal(claimResult.status, 1);
  assert.match(claimResult.stderr, /CODER_MODEL is required when Status is Ready for Dev on ORCHESTRATOR_MANAGED packets with an assigned EXECUTION_OWNER/i);
  assert.match(claimResult.stderr, /CODER_REASONING_STRENGTH is required when Status is Ready for Dev on ORCHESTRATOR_MANAGED packets with an assigned EXECUTION_OWNER/i);
});

test("orchestrator-managed ready packets with an assigned coder pass claim check when creation preclaims the governed session policy", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "packet-law-regression-governed-ready-pass-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const packetPath = path.join(govRoot, "task_packets", "WP-TEST-GOVERNED-READY-PASS-v1", "packet.md");

  writeFile(
    packetPath,
    [
      "# WP-TEST-GOVERNED-READY-PASS-v1",
      "",
      "- **Status:** Ready for Dev",
      "- PACKET_FORMAT_VERSION: 2026-04-01",
      "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
      "- EXECUTION_OWNER: CODER_A",
      "- CODER_MODEL: gpt-5.4",
      "- CODER_REASONING_STRENGTH: EXTRA_HIGH",
      "- TOUCHED_FILE_BUDGET: 1",
      "- BROAD_TOOL_ALLOWLIST: NONE",
      "- DATA_CONTRACT_PROFILE: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
      "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A",
      "- PACKET_WIDENING_DECISION: NONE",
      "- PACKET_WIDENING_EVIDENCE: N/A",
      "",
      "## IN_SCOPE_PATHS",
      "- .GOV/roles_shared/checks/task-packet-claim-check.mjs",
      "",
      "## OUT_OF_SCOPE",
      "- src/backend/handshake_core/src/locus/types.rs",
      "",
      formatDataContractDecisionSection({
        decision: "WAIVED_NOT_DATA_BEARING",
        reason: "The packet scope is governance-only and does not introduce governed product data surfaces.",
        evidence: ["IN_SCOPE_PATH: .GOV/roles_shared/checks/task-packet-claim-check.mjs"],
      }).trim(),
      "",
      formatDataContractMonitoringSection({
        profile: "NONE",
        inScopePaths: [],
      }).trim(),
      "",
    ].join("\n"),
  );

  const claimResult = runNode(claimCheckPath, govRoot);
  assert.equal(claimResult.status, 0, claimResult.stderr || claimResult.stdout);
});

test("2026-04-01 packet law rejects explicit waiver when in-scope paths are data-bearing", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "packet-law-regression-waiver-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const packetPath = path.join(govRoot, "task_packets", "WP-TEST-WAIVER-v1", "packet.md");

  writeFile(
    packetPath,
    [
      "# WP-TEST-WAIVER-v1",
      "",
      "- **Status:** Ready for Dev",
      "- PACKET_FORMAT_VERSION: 2026-04-01",
      "- TOUCHED_FILE_BUDGET: 1",
      "- BROAD_TOOL_ALLOWLIST: NONE",
      "- DATA_CONTRACT_PROFILE: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
      "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A",
      "- PACKET_WIDENING_DECISION: NONE",
      "- PACKET_WIDENING_EVIDENCE: N/A",
      "",
      "## IN_SCOPE_PATHS",
      "- src/backend/handshake_core/src/locus/types.rs",
      "",
      "## OUT_OF_SCOPE",
      "- src/frontend/app.tsx",
      "",
      formatDataContractDecisionSection({
        decision: "WAIVED_NOT_DATA_BEARING",
        reason: "The packet was initially believed not to touch governed data surfaces.",
        evidence: ["IN_SCOPE_PATHS reviewed: src/backend/handshake_core/src/locus/types.rs"],
      }).trim(),
      "",
      formatDataContractMonitoringSection({
        profile: "NONE",
        inScopePaths: [],
      }).trim(),
      "",
    ].join("\n"),
  );

  const claimResult = runNode(claimCheckPath, govRoot);
  assert.equal(claimResult.status, 1);
  assert.match(claimResult.stderr, /WAIVED_NOT_DATA_BEARING conflicts with data-bearing IN_SCOPE_PATHS/i);
});

test("legacy packet family remains grandfathered without explicit data-contract decision", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "packet-law-regression-legacy-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const packetPath = path.join(govRoot, "task_packets", "WP-TEST-LEGACY-v1", "packet.md");

  writeFile(
    packetPath,
    [
      "# WP-TEST-LEGACY-v1",
      "",
      "- **Status:** Ready for Dev",
      "- PACKET_FORMAT_VERSION: 2026-03-31",
      "- TOUCHED_FILE_BUDGET: 1",
      "- BROAD_TOOL_ALLOWLIST: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
      "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE",
      "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A",
      "- PACKET_WIDENING_DECISION: NONE",
      "- PACKET_WIDENING_EVIDENCE: N/A",
      "",
      "## IN_SCOPE_PATHS",
      "- src/backend/handshake_core/src/locus/types.rs",
      "",
      "## OUT_OF_SCOPE",
      "- src/frontend/app.tsx",
      "",
    ].join("\n"),
  );

  const claimResult = runNode(claimCheckPath, govRoot);
  assert.equal(claimResult.status, 0, claimResult.stderr || claimResult.stdout);
});
