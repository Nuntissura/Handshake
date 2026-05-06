import test from "node:test";
import assert from "node:assert/strict";
import {
  buildWorkflowContractEnvelope,
  buildWorkflowContractPromptCapsule,
  loadWorkflowContract,
  validateWorkflowContract,
  validateWorkflowContractEnvelopeShape,
} from "../scripts/workflow/workflow-contract-lib.mjs";

test("orchestrator-managed contract validates with 20+ failure classes", () => {
  const contract = loadWorkflowContract("ORCHESTRATOR_MANAGED", { useCache: false });
  assert.equal(contract.owner_roles[0], "ORCHESTRATOR");
  assert.ok(contract.consumers.includes("ACP_SESSION_CONTROL"));
  assert.ok(contract.failure_classes.length >= 20);
  assert.deepEqual(validateWorkflowContract(contract, {
    expectedLane: "ORCHESTRATOR_MANAGED",
    minFailureClasses: 20,
  }), []);
});

test("manual-relay contract is owned by Classic Orchestrator", () => {
  const contract = loadWorkflowContract("MANUAL_RELAY", { useCache: false });
  assert.equal(contract.owner_roles[0], "CLASSIC_ORCHESTRATOR");
  assert.ok(contract.consumers.includes("ACP_SESSION_CONTROL"));
  assert.deepEqual(validateWorkflowContract(contract, {
    expectedLane: "MANUAL_RELAY",
    minFailureClasses: 8,
  }), []);
});

test("workflow contract envelope is ACP-readable and role-scoped", () => {
  const envelope = buildWorkflowContractEnvelope({
    role: "WP_VALIDATOR",
    commandKind: "SEND_PROMPT",
  });
  assert.equal(envelope.schema_id, "hsk.workflow_contract_envelope@1");
  assert.equal(envelope.lane, "ORCHESTRATOR_MANAGED");
  assert.equal(envelope.role, "WP_VALIDATOR");
  assert.ok(envelope.allowed_next_commands.some((command) => command.includes("active-lane-brief")));
  assert.deepEqual(validateWorkflowContractEnvelopeShape(envelope), []);
});

test("workflow prompt capsule stays compact JSON", () => {
  const capsule = buildWorkflowContractPromptCapsule({
    role: "CLASSIC_ORCHESTRATOR",
    commandKind: "SEND_PROMPT",
  });
  assert.ok(capsule.length < 1200);
  const parsed = JSON.parse(capsule);
  assert.equal(parsed.schema_id, "hsk.workflow_contract_capsule@1");
  assert.equal(parsed.lane, "MANUAL_RELAY");
  assert.equal(parsed.role, "CLASSIC_ORCHESTRATOR");
});
