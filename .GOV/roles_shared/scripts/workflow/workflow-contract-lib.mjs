import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_ABS, GOV_ROOT_REPO_REL } from "../lib/runtime-paths.mjs";

export const WORKFLOW_CONTRACT_SCHEMA_ID = "hsk.workflow_contract@1";
export const WORKFLOW_CONTRACT_SCHEMA_VERSION = "workflow_contract_v1";
export const WORKFLOW_CONTRACT_ENVELOPE_SCHEMA_ID = "hsk.workflow_contract_envelope@1";
export const WORKFLOW_CONTRACT_ENVELOPE_SCHEMA_VERSION = "workflow_contract_envelope_v1";
export const WORKFLOW_CONTRACT_CAPSULE_SCHEMA_ID = "hsk.workflow_contract_capsule@1";
export const WORKFLOW_CONTRACT_CAPSULE_SCHEMA_VERSION = "workflow_contract_capsule_v1";

export const WORKFLOW_CONTRACT_LANES = Object.freeze([
  "ORCHESTRATOR_MANAGED",
  "MANUAL_RELAY",
]);

export const WORKFLOW_CONTRACT_PATHS = Object.freeze({
  ORCHESTRATOR_MANAGED: path.join(GOV_ROOT_REPO_REL, "roles_shared", "workflow_contracts", "orchestrator_managed.workflow.json"),
  MANUAL_RELAY: path.join(GOV_ROOT_REPO_REL, "roles_shared", "workflow_contracts", "manual_relay.workflow.json"),
});

const CONTRACT_CACHE = new Map();

function normalizeRole(role) {
  return String(role || "").trim().toUpperCase();
}

function normalizeLane(lane) {
  return String(lane || "").trim().toUpperCase();
}

function uniqueStrings(values) {
  return Array.from(new Set((Array.isArray(values) ? values : []).map((value) => String(value || "").trim()).filter(Boolean)));
}

function readJsonFile(absPath) {
  return JSON.parse(fs.readFileSync(absPath, "utf8"));
}

export function workflowContractPathForLane(lane) {
  const normalizedLane = normalizeLane(lane);
  const repoRelativePath = WORKFLOW_CONTRACT_PATHS[normalizedLane];
  if (!repoRelativePath) {
    throw new Error(`Unknown workflow contract lane: ${lane || "<empty>"}`);
  }
  return repoRelativePath;
}

export function workflowContractAbsPathForLane(lane) {
  const repoRelativePath = workflowContractPathForLane(lane);
  return path.resolve(GOV_ROOT_ABS, "..", repoRelativePath);
}

export function workflowContractLaneForRole(role) {
  const normalizedRole = normalizeRole(role);
  if (["CLASSIC_ORCHESTRATOR", "VALIDATOR"].includes(normalizedRole)) return "MANUAL_RELAY";
  return "ORCHESTRATOR_MANAGED";
}

export function loadWorkflowContract(lane, { useCache = true } = {}) {
  const normalizedLane = normalizeLane(lane);
  if (useCache && CONTRACT_CACHE.has(normalizedLane)) return CONTRACT_CACHE.get(normalizedLane);
  const absPath = workflowContractAbsPathForLane(normalizedLane);
  const contract = readJsonFile(absPath);
  if (useCache) CONTRACT_CACHE.set(normalizedLane, contract);
  return contract;
}

export function loadWorkflowContractForRole(role, { lane = "", useCache = true } = {}) {
  return loadWorkflowContract(lane || workflowContractLaneForRole(role), { useCache });
}

function addDuplicateErrors(errors, label, ids) {
  const seen = new Set();
  for (const id of ids) {
    if (seen.has(id)) errors.push(`${label} duplicate id: ${id}`);
    seen.add(id);
  }
}

export function validateWorkflowContract(contract, {
  expectedLane = "",
  minFailureClasses = 1,
} = {}) {
  const errors = [];
  if (!contract || typeof contract !== "object" || Array.isArray(contract)) {
    return ["contract must be an object"];
  }
  if (contract.schema_id !== WORKFLOW_CONTRACT_SCHEMA_ID) errors.push(`schema_id must be ${WORKFLOW_CONTRACT_SCHEMA_ID}`);
  if (contract.schema_version !== WORKFLOW_CONTRACT_SCHEMA_VERSION) errors.push(`schema_version must be ${WORKFLOW_CONTRACT_SCHEMA_VERSION}`);
  if (!String(contract.contract_id || "").trim()) errors.push("contract_id is required");
  if (!String(contract.contract_version || "").trim()) errors.push("contract_version is required");
  if (contract.status !== "ACTIVE") errors.push("status must be ACTIVE");
  if (contract.authority !== "MACHINE_CONTRACT") errors.push("authority must be MACHINE_CONTRACT");
  if (!WORKFLOW_CONTRACT_LANES.includes(contract.lane)) errors.push(`lane must be one of ${WORKFLOW_CONTRACT_LANES.join(", ")}`);
  if (expectedLane && contract.lane !== expectedLane) errors.push(`lane must be ${expectedLane}`);
  if (!uniqueStrings(contract.owner_roles).length) errors.push("owner_roles must include at least one role");
  if (!uniqueStrings(contract.maintainer_roles).length) errors.push("maintainer_roles must include at least one role");
  if (!uniqueStrings(contract.consumers).includes("ACP_SESSION_CONTROL")) errors.push("consumers must include ACP_SESSION_CONTROL");
  if (!contract.role_scopes || typeof contract.role_scopes !== "object" || Array.isArray(contract.role_scopes)) {
    errors.push("role_scopes must be an object");
  }

  const states = Array.isArray(contract.states) ? contract.states : [];
  const stateIds = states.map((entry) => String(entry?.id || "").trim()).filter(Boolean);
  if (stateIds.length === 0) errors.push("states must include at least one state id");
  addDuplicateErrors(errors, "states", stateIds);

  const failureClasses = Array.isArray(contract.failure_classes) ? contract.failure_classes : [];
  const failureClassIds = failureClasses.map((entry) => String(entry?.id || "").trim()).filter(Boolean);
  if (failureClassIds.length < minFailureClasses) {
    errors.push(`failure_classes must include at least ${minFailureClasses} entries`);
  }
  addDuplicateErrors(errors, "failure_classes", failureClassIds);
  for (const failureClass of failureClasses) {
    const id = String(failureClass?.id || "").trim();
    if (!id) continue;
    if (!String(failureClass.symptom || "").trim()) errors.push(`${id}: symptom is required`);
    if (!Array.isArray(failureClass.probes) || failureClass.probes.length === 0) errors.push(`${id}: probes must be non-empty`);
    if (!Array.isArray(failureClass.repairs) || failureClass.repairs.length === 0) errors.push(`${id}: repairs must be non-empty`);
  }

  const terminalFences = Array.isArray(contract.terminal_fences) ? contract.terminal_fences : [];
  if (terminalFences.length === 0) errors.push("terminal_fences must include at least one fence");
  addDuplicateErrors(errors, "terminal_fences", terminalFences.map((entry) => String(entry?.id || "").trim()).filter(Boolean));

  const scopeEntries = Object.entries(contract.role_scopes || {});
  for (const [role, scope] of scopeEntries) {
    if (!scope || typeof scope !== "object" || Array.isArray(scope)) {
      errors.push(`${role}: role scope must be an object`);
      continue;
    }
    if (!Array.isArray(scope.allowed_command_kinds) || scope.allowed_command_kinds.length === 0) {
      errors.push(`${role}: allowed_command_kinds must be non-empty`);
    }
    if (!Array.isArray(scope.allowed_next_commands) || scope.allowed_next_commands.length === 0) {
      errors.push(`${role}: allowed_next_commands must be non-empty`);
    }
    if (!Array.isArray(scope.forbidden_actions) || scope.forbidden_actions.length === 0) {
      errors.push(`${role}: forbidden_actions must be non-empty`);
    }
    for (const id of uniqueStrings(scope.failure_class_ids)) {
      if (!failureClassIds.includes(id)) errors.push(`${role}: unknown failure_class_id ${id}`);
    }
  }

  return errors;
}

export function validateWorkflowContractEnvelopeShape(envelope, { allowMissing = false } = {}) {
  if (!envelope) return allowMissing ? [] : ["workflow_contract is required"];
  const errors = [];
  if (typeof envelope !== "object" || Array.isArray(envelope)) return ["workflow_contract must be an object"];
  if (envelope.schema_id !== WORKFLOW_CONTRACT_ENVELOPE_SCHEMA_ID) errors.push(`workflow_contract.schema_id must be ${WORKFLOW_CONTRACT_ENVELOPE_SCHEMA_ID}`);
  if (envelope.schema_version !== WORKFLOW_CONTRACT_ENVELOPE_SCHEMA_VERSION) errors.push(`workflow_contract.schema_version must be ${WORKFLOW_CONTRACT_ENVELOPE_SCHEMA_VERSION}`);
  if (!String(envelope.contract_id || "").trim()) errors.push("workflow_contract.contract_id is required");
  if (!String(envelope.contract_version || "").trim()) errors.push("workflow_contract.contract_version is required");
  if (!WORKFLOW_CONTRACT_LANES.includes(envelope.lane)) errors.push(`workflow_contract.lane must be one of ${WORKFLOW_CONTRACT_LANES.join(", ")}`);
  if (!String(envelope.role || "").trim()) errors.push("workflow_contract.role is required");
  if (!uniqueStrings(envelope.owner_roles).length) errors.push("workflow_contract.owner_roles must be non-empty");
  if (!uniqueStrings(envelope.consumers).includes("ACP_SESSION_CONTROL")) errors.push("workflow_contract.consumers must include ACP_SESSION_CONTROL");
  if (!Array.isArray(envelope.allowed_command_kinds)) errors.push("workflow_contract.allowed_command_kinds must be an array");
  if (!Array.isArray(envelope.failure_class_ids)) errors.push("workflow_contract.failure_class_ids must be an array");
  if (!Array.isArray(envelope.terminal_fence_ids)) errors.push("workflow_contract.terminal_fence_ids must be an array");
  return errors;
}

export function buildWorkflowContractEnvelope({ role, lane = "", commandKind = "" } = {}) {
  const normalizedRole = normalizeRole(role);
  const contract = loadWorkflowContractForRole(normalizedRole, { lane });
  const roleScope = contract.role_scopes?.[normalizedRole] || {};
  return {
    schema_id: WORKFLOW_CONTRACT_ENVELOPE_SCHEMA_ID,
    schema_version: WORKFLOW_CONTRACT_ENVELOPE_SCHEMA_VERSION,
    contract_id: contract.contract_id,
    contract_version: contract.contract_version,
    lane: contract.lane,
    role: normalizedRole,
    command_kind: String(commandKind || "").trim().toUpperCase(),
    owner_roles: uniqueStrings(contract.owner_roles),
    maintainer_roles: uniqueStrings(contract.maintainer_roles),
    consumers: uniqueStrings(contract.consumers),
    role_scope_id: `${contract.contract_id}:${normalizedRole}`,
    allowed_command_kinds: uniqueStrings(roleScope.allowed_command_kinds),
    allowed_next_commands: uniqueStrings(roleScope.allowed_next_commands),
    forbidden_actions: uniqueStrings(roleScope.forbidden_actions),
    failure_class_ids: uniqueStrings(roleScope.failure_class_ids),
    terminal_fence_ids: uniqueStrings((contract.terminal_fences || []).map((entry) => entry?.id)),
    telemetry: contract.telemetry || {},
  };
}

export function buildWorkflowContractPromptCapsule({ role, lane = "", commandKind = "", maxFailureClasses = 5 } = {}) {
  const envelope = buildWorkflowContractEnvelope({ role, lane, commandKind });
  const capsule = {
    schema_id: WORKFLOW_CONTRACT_CAPSULE_SCHEMA_ID,
    schema_version: WORKFLOW_CONTRACT_CAPSULE_SCHEMA_VERSION,
    contract_id: envelope.contract_id,
    lane: envelope.lane,
    role: envelope.role,
    owner_roles: envelope.owner_roles,
    allowed_next_commands: envelope.allowed_next_commands.slice(0, 3),
    forbidden_actions: envelope.forbidden_actions.slice(0, 3),
    failure_class_ids: envelope.failure_class_ids.slice(0, maxFailureClasses),
    terminal_fence_ids: envelope.terminal_fence_ids.slice(0, 4),
  };
  return JSON.stringify(capsule);
}
