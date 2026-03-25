#!/usr/bin/env node

import fs from "node:fs";
import {
  currentGitContext,
  loadPacket,
  packetPath,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import {
  ensureValidatorGateDir,
  resolveValidatorGatePath,
} from "../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import {
  evaluateValidatorPacketGovernanceState,
  resolveValidatorActorContext,
} from "../scripts/lib/validator-governance-lib.mjs";
import { evaluateIntegrationValidatorCloseoutState } from "../scripts/lib/integration-validator-closeout-lib.mjs";

function fail(message, details = []) {
  console.error(`[INTEGRATION_VALIDATOR_CLOSEOUT_CHECK] FAIL: ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(1);
}

function pass(message, details = []) {
  console.log(`[INTEGRATION_VALIDATOR_CLOSEOUT_CHECK] PASS: ${message}`);
  for (const detail of details) console.log(`  ${detail}`);
}

function loadGateState(wpId) {
  ensureValidatorGateDir();
  const filePath = resolveValidatorGatePath(wpId);
  if (!fs.existsSync(filePath)) return {};
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

const wpId = String(process.argv[2] || "").trim();
if (!wpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(wpId)) {
  fail("Usage: just integration-validator-closeout-check WP-1-Example");
}

const repoRoot = process.cwd();
const packetContent = loadPacket(wpId);
const governanceState = evaluateValidatorPacketGovernanceState({
  wpId,
  packetPath: packetPath(wpId),
  packetContent,
});
if (!governanceState.allowValidationResume) {
  fail("Closeout preflight is blocked for this packet", [
    governanceState.message,
    `computed_policy_outcome=${governanceState.computedPolicy.outcome}`,
    `computed_policy_applicability=${governanceState.computedPolicy.applicability_reason || "APPLICABLE"}`,
  ]);
}

const gateState = loadGateState(wpId);
const committedEvidence = gateState?.committed_validation_evidence?.[wpId] || null;
const actorContext = resolveValidatorActorContext({
  repoRoot,
  wpId,
  packetContent,
  gitContext: currentGitContext(),
});
const evaluation = evaluateIntegrationValidatorCloseoutState({
  repoRoot,
  wpId,
  packetContent,
  actorContext,
  committedEvidence,
});

if (!evaluation.ok) {
  fail("Integration-validator topology or closeout bundle is not ready", [
    ...evaluation.issues,
  ]);
}

pass(`${wpId} final-lane topology and closeout bundle are coherent`, [
  `target_head_sha=${evaluation.topology.targetHeadSha || "<unknown>"}`,
  `integration_validator_worktree=${evaluation.topology.resolvedWorktreeAbs || "<unknown>"}`,
  `request_count=${evaluation.closeoutBundle.summary.request_count}`,
  `result_count=${evaluation.closeoutBundle.summary.result_count}`,
  `session_count=${evaluation.closeoutBundle.summary.session_count}`,
  `active_run_count=${evaluation.closeoutBundle.summary.active_run_count}`,
  `next=just validator-gate-commit ${wpId}`,
]);
