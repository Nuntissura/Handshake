function normalizeString(value, fallback = "<none>") {
  const text = String(value || "").trim();
  return text || fallback;
}

function parseNonNegativeInteger(value, fallback = 0) {
  const parsed = Number.parseInt(String(value ?? "").trim(), 10);
  if (!Number.isInteger(parsed) || parsed < 0) return fallback;
  return parsed;
}

export function normalizeRelayEscalationPolicy(value = null) {
  if (!value || typeof value !== "object") return null;
  return {
    source_surface: normalizeString(value.source_surface),
    failure_class: normalizeString(value.failure_class),
    policy_state: normalizeString(value.policy_state),
    next_strategy: normalizeString(value.next_strategy),
    reason_code: normalizeString(value.reason_code),
    budget_scope: normalizeString(value.budget_scope),
    budget_used: parseNonNegativeInteger(value.budget_used, 0),
    budget_limit: parseNonNegativeInteger(value.budget_limit, 0),
    summary: normalizeString(value.summary),
    updated_at: normalizeString(value.updated_at),
  };
}

export function relayEscalationPolicyBudgetLabel(policy = null, { noneLabel = "none" } = {}) {
  const normalized = normalizeRelayEscalationPolicy(policy);
  if (!normalized) return noneLabel;
  if (normalized.budget_scope === "NONE") return noneLabel;
  return `${normalized.budget_scope}:${normalized.budget_used}/${normalized.budget_limit}`;
}

export function relayEscalationPolicyStrategyLabel(policy = null) {
  const normalized = normalizeRelayEscalationPolicy(policy);
  if (!normalized) return "";
  return normalized.next_strategy.toLowerCase().replace(/_/g, "-");
}

export function relayEscalationPolicyInlineSummary(policy = null) {
  const normalized = normalizeRelayEscalationPolicy(policy);
  if (!normalized) return "";
  const fragments = [
    `failure_class=${normalized.failure_class}`,
    `policy=${normalized.policy_state}->${normalized.next_strategy}`,
  ];
  const budget = relayEscalationPolicyBudgetLabel(normalized);
  if (budget) fragments.push(`budget=${budget}`);
  return fragments.join(" | ");
}

