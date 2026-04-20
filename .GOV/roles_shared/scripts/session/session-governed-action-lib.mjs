import crypto from "node:crypto";

export const GOVERNED_ACTION_REQUEST_SCHEMA_ID = "hsk.governed_action_request@1";
export const GOVERNED_ACTION_REQUEST_SCHEMA_VERSION = "governed_action_request_v1";
export const GOVERNED_ACTION_RESULT_SCHEMA_ID = "hsk.governed_action_result@1";
export const GOVERNED_ACTION_RESULT_SCHEMA_VERSION = "governed_action_result_v1";
export const GOVERNED_ACTION_KINDS = Object.freeze([
  "APPROVE",
  "DENY",
  "DEFER",
  "RETRY",
  "SKIP",
  "EXTERNAL_EXECUTE",
]);
export const GOVERNED_ACTION_RESULT_STATES = Object.freeze([
  "REQUESTED",
  "ACCEPTED_PENDING",
  "SETTLED",
  "REJECTED",
  "DEFERRED",
  "SKIPPED",
  "FAILED",
]);
export const GOVERNED_ACTION_RESUME_DISPOSITIONS = Object.freeze([
  "PENDING",
  "RESUME_ALLOWED",
  "STOP",
  "DEFERRED",
  "RETRY_ALLOWED",
  "SKIP_ALLOWED",
  "CONSUME_RESULT",
  "REPAIR_REQUIRED",
]);

function nowIso() {
  return new Date().toISOString();
}

function isPlainObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function cleanString(value) {
  return String(value ?? "").trim();
}

function normalizeMetadata(value) {
  if (!isPlainObject(value)) return {};
  return Object.fromEntries(
    Object.entries(value)
      .map(([key, entryValue]) => [cleanString(key), entryValue])
      .filter(([key, entryValue]) => key && entryValue !== undefined),
  );
}

function freezeRule(rule = {}) {
  return Object.freeze({
    ...rule,
    created_by_roles: Object.freeze([...(rule.created_by_roles || [])]),
  });
}

export const GOVERNED_ACTION_RULE_REGISTRY = Object.freeze({
  SESSION_CONTROL_START_SESSION_EXTERNAL_EXECUTE: freezeRule({
    rule_id: "SESSION_CONTROL_START_SESSION_EXTERNAL_EXECUTE",
    action_surface: "SESSION_CONTROL",
    action_kind: "EXTERNAL_EXECUTE",
    command_kind: "START_SESSION",
    created_by_roles: ["ORCHESTRATOR"],
    requires_target_command_id: false,
    resume_policy: "WAIT_FOR_TRANSPORT_RESULT",
    default_reason_code: "START_SESSION",
    summary_template: "Start the governed session through the ACP session-control lane.",
  }),
  SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE: freezeRule({
    rule_id: "SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE",
    action_surface: "SESSION_CONTROL",
    action_kind: "EXTERNAL_EXECUTE",
    command_kind: "SEND_PROMPT",
    created_by_roles: ["ORCHESTRATOR"],
    requires_target_command_id: false,
    resume_policy: "WAIT_FOR_TRANSPORT_RESULT",
    default_reason_code: "SEND_PROMPT",
    summary_template: "Resume the governed session through the ACP session-control lane.",
  }),
  SESSION_CONTROL_CANCEL_SESSION_EXTERNAL_EXECUTE: freezeRule({
    rule_id: "SESSION_CONTROL_CANCEL_SESSION_EXTERNAL_EXECUTE",
    action_surface: "SESSION_CONTROL",
    action_kind: "EXTERNAL_EXECUTE",
    command_kind: "CANCEL_SESSION",
    created_by_roles: ["ORCHESTRATOR"],
    requires_target_command_id: true,
    resume_policy: "WAIT_FOR_TRANSPORT_RESULT",
    default_reason_code: "CANCEL_SESSION",
    summary_template: "Cancel the active governed session command through the ACP session-control lane.",
  }),
  SESSION_CONTROL_CLOSE_SESSION_EXTERNAL_EXECUTE: freezeRule({
    rule_id: "SESSION_CONTROL_CLOSE_SESSION_EXTERNAL_EXECUTE",
    action_surface: "SESSION_CONTROL",
    action_kind: "EXTERNAL_EXECUTE",
    command_kind: "CLOSE_SESSION",
    created_by_roles: ["ORCHESTRATOR"],
    requires_target_command_id: false,
    resume_policy: "WAIT_FOR_TRANSPORT_RESULT",
    default_reason_code: "CLOSE_SESSION",
    summary_template: "Close the governed session through the ACP session-control lane.",
  }),
  VALIDATOR_GATE_APPROVE_RESUME: freezeRule({
    rule_id: "VALIDATOR_GATE_APPROVE_RESUME",
    action_surface: "VALIDATOR_GATE",
    action_kind: "APPROVE",
    command_kind: "",
    created_by_roles: ["ORCHESTRATOR", "VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "RESUME_ON_SETTLED",
    default_reason_code: "APPROVE_RESUME",
    summary_template: "Approve the next governed resume step without reconstructing intent from transcript prose.",
  }),
  VALIDATOR_GATE_DENY_RESUME: freezeRule({
    rule_id: "VALIDATOR_GATE_DENY_RESUME",
    action_surface: "VALIDATOR_GATE",
    action_kind: "DENY",
    command_kind: "",
    created_by_roles: ["ORCHESTRATOR", "VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "STOP_ON_SETTLED",
    default_reason_code: "DENY_RESUME",
    summary_template: "Deny the next governed resume step and keep the lane blocked.",
  }),
  VALIDATOR_GATE_DEFER_RESUME: freezeRule({
    rule_id: "VALIDATOR_GATE_DEFER_RESUME",
    action_surface: "VALIDATOR_GATE",
    action_kind: "DEFER",
    command_kind: "",
    created_by_roles: ["ORCHESTRATOR", "VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "REQUEUE_LATER",
    default_reason_code: "DEFER_RESUME",
    summary_template: "Defer the next governed resume step until another blocker clears.",
  }),
  VALIDATOR_GATE_RETRY_RESUME: freezeRule({
    rule_id: "VALIDATOR_GATE_RETRY_RESUME",
    action_surface: "VALIDATOR_GATE",
    action_kind: "RETRY",
    command_kind: "",
    created_by_roles: ["ORCHESTRATOR", "VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "RETRY_AFTER_SETTLED",
    default_reason_code: "RETRY_RESUME",
    summary_template: "Retry the governed resume step with an explicit strategy change.",
  }),
  VALIDATOR_GATE_SKIP_RESUME: freezeRule({
    rule_id: "VALIDATOR_GATE_SKIP_RESUME",
    action_surface: "VALIDATOR_GATE",
    action_kind: "SKIP",
    command_kind: "",
    created_by_roles: ["ORCHESTRATOR", "VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "CONTINUE_AFTER_SETTLED",
    default_reason_code: "SKIP_RESUME",
    summary_template: "Skip the gated resume step and advance to the next governed state.",
  }),
  VALIDATOR_GATE_APPEND_APPROVE: freezeRule({
    rule_id: "VALIDATOR_GATE_APPEND_APPROVE",
    action_surface: "VALIDATOR_GATE",
    action_kind: "APPROVE",
    command_kind: "APPEND",
    created_by_roles: ["VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "RESUME_ON_SETTLED",
    default_reason_code: "APPEND_GATE",
    summary_template: "Record validator gate append progression in the governed ledger.",
  }),
  VALIDATOR_GATE_COMMIT_APPROVE: freezeRule({
    rule_id: "VALIDATOR_GATE_COMMIT_APPROVE",
    action_surface: "VALIDATOR_GATE",
    action_kind: "APPROVE",
    command_kind: "COMMIT",
    created_by_roles: ["VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "RESUME_ON_SETTLED",
    default_reason_code: "COMMIT_GATE",
    summary_template: "Record validator gate commit clearance in the governed ledger.",
  }),
  VALIDATOR_GATE_PRESENT_APPROVE: freezeRule({
    rule_id: "VALIDATOR_GATE_PRESENT_APPROVE",
    action_surface: "VALIDATOR_GATE",
    action_kind: "APPROVE",
    command_kind: "PRESENT_REPORT",
    created_by_roles: ["VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "RESUME_ON_SETTLED",
    default_reason_code: "PRESENT_GATE",
    summary_template: "Record validator gate report presentation in the governed ledger.",
  }),
  VALIDATOR_GATE_ACKNOWLEDGE_APPROVE: freezeRule({
    rule_id: "VALIDATOR_GATE_ACKNOWLEDGE_APPROVE",
    action_surface: "VALIDATOR_GATE",
    action_kind: "APPROVE",
    command_kind: "ACKNOWLEDGE",
    created_by_roles: ["VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "RESUME_ON_SETTLED",
    default_reason_code: "ACKNOWLEDGE_GATE",
    summary_template: "Record validator gate acknowledgment progression in the governed ledger.",
  }),
  VALIDATOR_GATE_RESET_EXTERNAL_EXECUTE: freezeRule({
    rule_id: "VALIDATOR_GATE_RESET_EXTERNAL_EXECUTE",
    action_surface: "VALIDATOR_GATE",
    action_kind: "EXTERNAL_EXECUTE",
    command_kind: "RESET",
    created_by_roles: ["VALIDATOR", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "CONTINUE_AFTER_SETTLED",
    default_reason_code: "RESET_GATE",
    summary_template: "Reset the validator gate ledger through a direct governed write.",
  }),
  INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE: freezeRule({
    rule_id: "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE",
    action_surface: "INTEGRATION_VALIDATOR_CLOSEOUT",
    action_kind: "EXTERNAL_EXECUTE",
    command_kind: "CLOSEOUT_SYNC",
    created_by_roles: ["INTEGRATION_VALIDATOR"],
    requires_target_command_id: false,
    resume_policy: "CONTINUE_AFTER_SETTLED",
    default_reason_code: "CLOSEOUT_SYNC",
    summary_template: "Record the final-lane closeout sync through a typed governed external-execute action.",
  }),
});
export const GOVERNED_ACTION_RULE_IDS = Object.freeze(Object.keys(GOVERNED_ACTION_RULE_REGISTRY));

export function governedActionRule(ruleId = "") {
  return GOVERNED_ACTION_RULE_REGISTRY[cleanString(ruleId).toUpperCase()] || null;
}

export function defaultGovernedActionRuleIdForSessionCommand(commandKind = "", actionKind = "EXTERNAL_EXECUTE") {
  const normalizedActionKind = cleanString(actionKind).toUpperCase();
  const normalizedCommandKind = cleanString(commandKind).toUpperCase();
  if (normalizedActionKind !== "EXTERNAL_EXECUTE") return "";
  if (!normalizedCommandKind) return "";
  return `SESSION_CONTROL_${normalizedCommandKind}_EXTERNAL_EXECUTE`;
}

function resolveGovernedActionRule({ ruleId = "", actionKind = "", commandKind = "" } = {}) {
  const explicitRule = governedActionRule(ruleId);
  if (explicitRule) {
    if (actionKind && explicitRule.action_kind !== cleanString(actionKind).toUpperCase()) {
      throw new Error(`Governed action rule ${explicitRule.rule_id} does not allow action_kind ${actionKind}`);
    }
    if (commandKind && explicitRule.command_kind && explicitRule.command_kind !== cleanString(commandKind).toUpperCase()) {
      throw new Error(`Governed action rule ${explicitRule.rule_id} does not allow command_kind ${commandKind}`);
    }
    return explicitRule;
  }

  const derivedRuleId = defaultGovernedActionRuleIdForSessionCommand(commandKind, actionKind || "EXTERNAL_EXECUTE");
  const derivedRule = governedActionRule(derivedRuleId);
  if (derivedRule) return derivedRule;

  throw new Error(`Unknown governed action rule: ${ruleId || `<derived:${actionKind || "EXTERNAL_EXECUTE"}:${commandKind || "NONE"}>`}`);
}

export function classifyGovernedActionResultState({
  actionKind = "EXTERNAL_EXECUTE",
  status = "",
} = {}) {
  const normalizedActionKind = cleanString(actionKind).toUpperCase();
  const normalizedStatus = cleanString(status).toUpperCase();
  if (normalizedStatus === "QUEUED" || normalizedStatus === "RUNNING") return "ACCEPTED_PENDING";
  if (normalizedStatus === "FAILED") return "FAILED";
  if (normalizedStatus !== "COMPLETED") return "FAILED";
  if (normalizedActionKind === "DENY") return "REJECTED";
  if (normalizedActionKind === "DEFER") return "DEFERRED";
  if (normalizedActionKind === "SKIP") return "SKIPPED";
  return "SETTLED";
}

export function classifyGovernedActionResumeDisposition({
  actionKind = "EXTERNAL_EXECUTE",
  resultState = "",
} = {}) {
  const normalizedActionKind = cleanString(actionKind).toUpperCase();
  const normalizedResultState = cleanString(resultState).toUpperCase();
  if (normalizedResultState === "REQUESTED" || normalizedResultState === "ACCEPTED_PENDING") return "PENDING";
  if (normalizedResultState === "FAILED") return "REPAIR_REQUIRED";
  if (normalizedActionKind === "APPROVE") return "RESUME_ALLOWED";
  if (normalizedActionKind === "DENY") return "STOP";
  if (normalizedActionKind === "DEFER") return "DEFERRED";
  if (normalizedActionKind === "RETRY") return "RETRY_ALLOWED";
  if (normalizedActionKind === "SKIP") return "SKIP_ALLOWED";
  return "CONSUME_RESULT";
}

export function buildGovernedActionRequest({
  actionId = "",
  ruleId = "",
  actionKind = "",
  commandKind = "",
  commandId = "",
  sessionKey = "",
  wpId = "",
  role = "",
  createdByRole = "ORCHESTRATOR",
  targetCommandId = "",
  summary = "",
  reasonCode = "",
  metadata = {},
  requestedAt = "",
} = {}) {
  const rule = resolveGovernedActionRule({ ruleId, actionKind, commandKind });
  return {
    schema_id: GOVERNED_ACTION_REQUEST_SCHEMA_ID,
    schema_version: GOVERNED_ACTION_REQUEST_SCHEMA_VERSION,
    action_id: cleanString(actionId || commandId || crypto.randomUUID()),
    requested_at: cleanString(requestedAt || nowIso()),
    rule_id: rule.rule_id,
    action_kind: rule.action_kind,
    action_surface: rule.action_surface,
    command_kind: cleanString(commandKind).toUpperCase() || rule.command_kind || "",
    command_id: cleanString(commandId),
    created_by_role: cleanString(createdByRole).toUpperCase(),
    session_key: cleanString(sessionKey),
    wp_id: cleanString(wpId),
    role: cleanString(role).toUpperCase(),
    target_command_id: cleanString(targetCommandId),
    reason_code: cleanString(reasonCode) || rule.default_reason_code || "",
    summary: cleanString(summary) || rule.summary_template || "",
    resume_policy: rule.resume_policy,
    metadata: normalizeMetadata(metadata),
  };
}

export function buildGovernedActionResult({
  actionId = "",
  ruleId = "",
  actionKind = "",
  commandKind = "",
  commandId = "",
  sessionKey = "",
  wpId = "",
  role = "",
  status = "",
  outcomeState = "",
  targetCommandId = "",
  summary = "",
  error = "",
  metadata = {},
  processedAt = "",
} = {}) {
  const rule = resolveGovernedActionRule({ ruleId, actionKind, commandKind });
  const normalizedStatus = cleanString(status).toUpperCase();
  const resultState = classifyGovernedActionResultState({
    actionKind: rule.action_kind,
    status: normalizedStatus,
  });
  return {
    schema_id: GOVERNED_ACTION_RESULT_SCHEMA_ID,
    schema_version: GOVERNED_ACTION_RESULT_SCHEMA_VERSION,
    action_id: cleanString(actionId || commandId || crypto.randomUUID()),
    processed_at: cleanString(processedAt || nowIso()),
    rule_id: rule.rule_id,
    action_kind: rule.action_kind,
    action_surface: rule.action_surface,
    command_kind: cleanString(commandKind).toUpperCase() || rule.command_kind || "",
    command_id: cleanString(commandId),
    session_key: cleanString(sessionKey),
    wp_id: cleanString(wpId),
    role: cleanString(role).toUpperCase(),
    status: normalizedStatus,
    outcome_state: cleanString(outcomeState).toUpperCase(),
    result_state: resultState,
    resume_disposition: classifyGovernedActionResumeDisposition({
      actionKind: rule.action_kind,
      resultState,
    }),
    target_command_id: cleanString(targetCommandId),
    summary: cleanString(summary),
    error: cleanString(error),
    metadata: normalizeMetadata(metadata),
  };
}

function validateRuleBinding(action, errors) {
  const rule = governedActionRule(action?.rule_id);
  if (!rule) {
    errors.push(`rule_id is invalid (${action?.rule_id || "<missing>"})`);
    return null;
  }
  if (rule.action_kind !== cleanString(action?.action_kind).toUpperCase()) {
    errors.push(`rule ${rule.rule_id} requires action_kind ${rule.action_kind}`);
  }
  if (rule.command_kind && rule.command_kind !== cleanString(action?.command_kind).toUpperCase()) {
    errors.push(`rule ${rule.rule_id} requires command_kind ${rule.command_kind}`);
  }
  if (rule.action_surface !== cleanString(action?.action_surface).toUpperCase()) {
    errors.push(`rule ${rule.rule_id} requires action_surface ${rule.action_surface}`);
  }
  if (cleanString(action?.created_by_role) && !rule.created_by_roles.includes(cleanString(action.created_by_role).toUpperCase())) {
    errors.push(`rule ${rule.rule_id} does not allow created_by_role ${action.created_by_role}`);
  }
  if (rule.requires_target_command_id && !cleanString(action?.target_command_id)) {
    errors.push(`rule ${rule.rule_id} requires target_command_id`);
  }
  return rule;
}

export function validateGovernedActionRequestShape(action, { allowMissing = false } = {}) {
  if (!action) return allowMissing ? [] : ["governed_action is required"];
  const errors = [];
  if (!isPlainObject(action)) return ["governed_action must be an object"];
  if (action.schema_id !== GOVERNED_ACTION_REQUEST_SCHEMA_ID) {
    errors.push(`schema_id must be ${GOVERNED_ACTION_REQUEST_SCHEMA_ID}`);
  }
  if (action.schema_version !== GOVERNED_ACTION_REQUEST_SCHEMA_VERSION) {
    errors.push(`schema_version must be ${GOVERNED_ACTION_REQUEST_SCHEMA_VERSION}`);
  }
  if (!cleanString(action.action_id)) errors.push("action_id is required");
  if (!cleanString(action.requested_at)) errors.push("requested_at is required");
  if (!GOVERNED_ACTION_KINDS.includes(cleanString(action.action_kind).toUpperCase())) {
    errors.push("action_kind is invalid");
  }
  if (!cleanString(action.command_id)) errors.push("command_id is required");
  if (!cleanString(action.created_by_role)) errors.push("created_by_role is required");
  if (!cleanString(action.session_key)) errors.push("session_key is required");
  if (!cleanString(action.wp_id)) errors.push("wp_id is required");
  if (!cleanString(action.role)) errors.push("role is required");
  if ("metadata" in action && (!isPlainObject(action.metadata))) {
    errors.push("metadata must be an object when present");
  }
  const rule = validateRuleBinding(action, errors);
  if (rule && cleanString(action.resume_policy) !== rule.resume_policy) {
    errors.push(`rule ${rule.rule_id} requires resume_policy ${rule.resume_policy}`);
  }
  return errors;
}

export function validateGovernedActionResultShape(action, { allowMissing = false } = {}) {
  if (!action) return allowMissing ? [] : ["governed_action is required"];
  const errors = [];
  if (!isPlainObject(action)) return ["governed_action must be an object"];
  if (action.schema_id !== GOVERNED_ACTION_RESULT_SCHEMA_ID) {
    errors.push(`schema_id must be ${GOVERNED_ACTION_RESULT_SCHEMA_ID}`);
  }
  if (action.schema_version !== GOVERNED_ACTION_RESULT_SCHEMA_VERSION) {
    errors.push(`schema_version must be ${GOVERNED_ACTION_RESULT_SCHEMA_VERSION}`);
  }
  if (!cleanString(action.action_id)) errors.push("action_id is required");
  if (!cleanString(action.processed_at)) errors.push("processed_at is required");
  if (!GOVERNED_ACTION_KINDS.includes(cleanString(action.action_kind).toUpperCase())) {
    errors.push("action_kind is invalid");
  }
  if (!cleanString(action.command_id)) errors.push("command_id is required");
  if (!cleanString(action.session_key)) errors.push("session_key is required");
  if (!cleanString(action.wp_id)) errors.push("wp_id is required");
  if (!cleanString(action.role)) errors.push("role is required");
  if (!cleanString(action.status)) errors.push("status is required");
  if (!GOVERNED_ACTION_RESULT_STATES.includes(cleanString(action.result_state).toUpperCase())) {
    errors.push("result_state is invalid");
  }
  if (!GOVERNED_ACTION_RESUME_DISPOSITIONS.includes(cleanString(action.resume_disposition).toUpperCase())) {
    errors.push("resume_disposition is invalid");
  }
  if ("metadata" in action && (!isPlainObject(action.metadata))) {
    errors.push("metadata must be an object when present");
  }
  validateRuleBinding(action, errors);
  return errors;
}

export function summarizeGovernedAction(action = {}) {
  if (!isPlainObject(action) || !cleanString(action.action_id)) return null;
  const requestedAt = cleanString(action.requested_at);
  const processedAt = cleanString(action.processed_at);
  return {
    action_id: cleanString(action.action_id),
    rule_id: cleanString(action.rule_id),
    action_kind: cleanString(action.action_kind).toUpperCase() || "NONE",
    action_surface: cleanString(action.action_surface).toUpperCase() || "NONE",
    command_kind: cleanString(action.command_kind).toUpperCase() || "NONE",
    command_id: cleanString(action.command_id),
    action_state: cleanString(action.action_state).toUpperCase()
      || cleanString(action.result_state).toUpperCase()
      || "REQUESTED",
    status: cleanString(action.status).toUpperCase() || "REQUESTED",
    outcome_state: cleanString(action.outcome_state).toUpperCase(),
    resume_disposition: cleanString(action.resume_disposition).toUpperCase()
      || (processedAt ? "CONSUME_RESULT" : "PENDING"),
    target_command_id: cleanString(action.target_command_id),
    reason_code: cleanString(action.reason_code),
    summary: cleanString(action.summary),
    requested_at: requestedAt,
    processed_at: processedAt,
    updated_at: processedAt || requestedAt,
  };
}

function legacyActionStateFromStatus(status = "") {
  const normalizedStatus = cleanString(status).toUpperCase();
  if (normalizedStatus === "QUEUED") return "REQUESTED";
  if (normalizedStatus === "RUNNING") return "ACCEPTED_PENDING";
  if (normalizedStatus === "COMPLETED") return "SETTLED";
  if (normalizedStatus === "FAILED") return "FAILED";
  return "NONE";
}

function buildLegacySessionGovernedAction(session = {}) {
  const commandId = cleanString(session?.last_command_id);
  const commandKind = cleanString(session?.last_command_kind).toUpperCase() || "NONE";
  const status = cleanString(session?.last_command_status).toUpperCase() || "NONE";
  const summary = cleanString(session?.last_command_summary);
  if (!commandId && commandKind === "NONE" && status === "NONE" && !summary) return null;

  const actionKind = commandKind !== "NONE" ? "EXTERNAL_EXECUTE" : "NONE";
  const resultState = actionKind === "EXTERNAL_EXECUTE"
    ? classifyGovernedActionResultState({ actionKind, status })
    : "REQUESTED";
  const updatedAt = cleanString(session?.last_command_completed_at)
    || cleanString(session?.last_command_prompt_at);

  return {
    action_id: commandId || "",
    rule_id: actionKind === "EXTERNAL_EXECUTE"
      ? defaultGovernedActionRuleIdForSessionCommand(commandKind, actionKind)
      : "",
    action_kind: actionKind,
    action_surface: actionKind === "EXTERNAL_EXECUTE" ? "SESSION_CONTROL" : "NONE",
    command_kind: commandKind,
    command_id: commandId,
    action_state: legacyActionStateFromStatus(status),
    status,
    outcome_state: status === "COMPLETED" ? "SETTLED" : "",
    resume_disposition: actionKind === "EXTERNAL_EXECUTE"
      ? classifyGovernedActionResumeDisposition({ actionKind, resultState })
      : "PENDING",
    target_command_id: "",
    reason_code: "",
    summary,
    requested_at: cleanString(session?.last_command_prompt_at),
    processed_at: cleanString(session?.last_command_completed_at),
    updated_at: updatedAt,
  };
}

export function effectiveSessionGovernedAction(session = {}) {
  const directAction = summarizeGovernedAction(session?.last_governed_action);
  const history = Array.isArray(session?.action_history) ? session.action_history : [];
  const historyAction = history.length > 0 ? summarizeGovernedAction(history[history.length - 1]) : null;
  const effective = directAction || historyAction || buildLegacySessionGovernedAction(session);
  if (!effective) return null;

  return {
    ...effective,
    source: directAction || historyAction ? "governed_action" : "legacy_command_fields",
    legacy_command_id: cleanString(session?.last_command_id),
    legacy_command_kind: cleanString(session?.last_command_kind).toUpperCase() || "NONE",
    legacy_command_status: cleanString(session?.last_command_status).toUpperCase() || "NONE",
  };
}

export function governedSessionMirrorDrift(session = {}) {
  const effective = effectiveSessionGovernedAction(session);
  if (!effective || effective.source !== "governed_action") return [];

  const drifts = [];
  const lastCommandId = cleanString(session?.last_command_id);
  const lastCommandKind = cleanString(session?.last_command_kind).toUpperCase();
  const lastCommandStatus = cleanString(session?.last_command_status).toUpperCase();

  if (lastCommandId && effective.command_id && lastCommandId !== effective.command_id) {
    drifts.push(`effective governed command_id disagrees with last_command_id (${effective.command_id} vs ${lastCommandId})`);
  }
  if (lastCommandKind && lastCommandKind !== "NONE" && effective.command_kind && lastCommandKind !== effective.command_kind) {
    drifts.push(`effective governed command_kind disagrees with last_command_kind (${effective.command_kind} vs ${lastCommandKind})`);
  }
  if (
    lastCommandStatus
    && !(lastCommandStatus === "NONE" && effective.status === "REQUESTED")
    && effective.status
    && lastCommandStatus !== effective.status
  ) {
    drifts.push(`effective governed status disagrees with last_command_status (${effective.status} vs ${lastCommandStatus})`);
  }
  return drifts;
}
