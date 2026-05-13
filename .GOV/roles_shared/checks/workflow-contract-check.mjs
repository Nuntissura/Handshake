import {
  WORKFLOW_CONTRACT_PATHS,
  buildWorkflowContractEnvelope,
  buildWorkflowContractPromptCapsule,
  loadWorkflowContract,
  validateWorkflowContract,
  validateWorkflowContractEnvelopeShape,
} from "../scripts/workflow/workflow-contract-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("workflow-contract-check.mjs", { role: "SHARED" });

const errors = [];

const laneMinimums = {
  ORCHESTRATOR_MANAGED: 20,
  MANUAL_RELAY: 8,
};

for (const lane of Object.keys(WORKFLOW_CONTRACT_PATHS)) {
  let contract;
  try {
    contract = loadWorkflowContract(lane, { useCache: false });
  } catch (error) {
    errors.push(`${lane}: failed to load workflow contract: ${error.message || error}`);
    continue;
  }
  errors.push(...validateWorkflowContract(contract, {
    expectedLane: lane,
    minFailureClasses: laneMinimums[lane] || 1,
  }).map((error) => `${lane}: ${error}`));
}

const envelopeChecks = [
  { role: "ORCHESTRATOR", lane: "ORCHESTRATOR_MANAGED", commandKind: "SEND_PROMPT" },
  { role: "ACTIVATION_MANAGER", lane: "ORCHESTRATOR_MANAGED", commandKind: "START_SESSION" },
  { role: "CODER", lane: "ORCHESTRATOR_MANAGED", commandKind: "SEND_PROMPT" },
  { role: "WP_VALIDATOR", lane: "ORCHESTRATOR_MANAGED", commandKind: "SEND_PROMPT" },
  { role: "INTEGRATION_VALIDATOR", lane: "ORCHESTRATOR_MANAGED", commandKind: "START_SESSION" },
  { role: "MEMORY_MANAGER", lane: "ORCHESTRATOR_MANAGED", commandKind: "START_SESSION" },
  { role: "CLASSIC_ORCHESTRATOR", lane: "MANUAL_RELAY", commandKind: "SEND_PROMPT" },
  { role: "VALIDATOR", lane: "MANUAL_RELAY", commandKind: "START_SESSION" },
];

for (const check of envelopeChecks) {
  const envelope = buildWorkflowContractEnvelope(check);
  errors.push(...validateWorkflowContractEnvelopeShape(envelope).map((error) => `${check.role}: ${error}`));
  const capsule = buildWorkflowContractPromptCapsule(check);
  try {
    const parsed = JSON.parse(capsule);
    if (parsed.contract_id !== envelope.contract_id) errors.push(`${check.role}: prompt capsule contract_id drift`);
    if (parsed.role !== check.role) errors.push(`${check.role}: prompt capsule role drift`);
  } catch (error) {
    errors.push(`${check.role}: prompt capsule is not JSON: ${error.message || error}`);
  }
}

if (errors.length > 0) {
  failWithMemory("workflow-contract-check.mjs", "Workflow contract validation failed", {
    role: "SHARED",
    details: errors,
  });
}

console.log("workflow-contract-check ok");
