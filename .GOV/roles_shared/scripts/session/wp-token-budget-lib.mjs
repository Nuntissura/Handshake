import {
  WP_ROLE_TOKEN_BUDGETS,
  WP_TOKEN_BUDGET_POLICY_ID,
  WP_TOTAL_TOKEN_BUDGET,
} from "./session-policy.mjs";

function normalizeCount(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric < 0) return 0;
  return Math.trunc(numeric);
}

function normalizeRole(role) {
  return String(role || "").trim().toUpperCase() || "UNKNOWN";
}

function metricStatus(actual, warnLimit, failLimit) {
  const normalizedActual = normalizeCount(actual);
  const normalizedWarn = normalizeCount(warnLimit);
  const normalizedFail = normalizeCount(failLimit);
  if (normalizedFail > 0 && normalizedActual > normalizedFail) return "FAIL";
  if (normalizedWarn > 0 && normalizedActual > normalizedWarn) return "WARN";
  return "PASS";
}

function worseStatus(left, right) {
  const rank = { PASS: 0, WARN: 1, FAIL: 2 };
  return (rank[right] || 0) > (rank[left] || 0) ? right : left;
}

function evaluateBudgetSlice(label, totals = {}, budget = {}) {
  const turnCount = normalizeCount(totals.turn_count);
  const inputTokens = normalizeCount(totals?.usage_totals?.input_tokens);
  const cachedInputTokens = normalizeCount(totals?.usage_totals?.cached_input_tokens);
  const freshInputTokens = Math.max(0, inputTokens - cachedInputTokens);
  const turnStatus = metricStatus(turnCount, budget.warn_turn_count, budget.fail_turn_count);
  const freshInputStatus = metricStatus(freshInputTokens, budget.warn_input_tokens, budget.fail_input_tokens);
  const grossInputStatus = metricStatus(inputTokens, budget.warn_input_tokens, budget.fail_input_tokens);
  const status = worseStatus(turnStatus, freshInputStatus);
  const warnings = [];
  const failures = [];

  if (turnStatus === "WARN") {
    warnings.push(`${label} turn_count ${turnCount} exceeded warn budget ${normalizeCount(budget.warn_turn_count)}`);
  } else if (turnStatus === "FAIL") {
    failures.push(`${label} turn_count ${turnCount} exceeded fail budget ${normalizeCount(budget.fail_turn_count)}`);
  }

  if (freshInputStatus === "WARN") {
    warnings.push(
      `${label} fresh_input_tokens ${freshInputTokens} exceeded warn budget ${normalizeCount(budget.warn_input_tokens)}`,
    );
  } else if (freshInputStatus === "FAIL") {
    failures.push(
      `${label} fresh_input_tokens ${freshInputTokens} exceeded fail budget ${normalizeCount(budget.fail_input_tokens)}`,
    );
  }

  if (grossInputStatus === "WARN" || grossInputStatus === "FAIL") {
    warnings.push(
      `${label} gross_input_tokens ${inputTokens} exceeded telemetry threshold ${grossInputStatus === "FAIL" ? normalizeCount(budget.fail_input_tokens) : normalizeCount(budget.warn_input_tokens)} with cached_input_tokens ${cachedInputTokens}`,
    );
  }

  return {
    label,
    status,
    turn_count: turnCount,
    input_tokens: inputTokens,
    cached_input_tokens: cachedInputTokens,
    fresh_input_tokens: freshInputTokens,
    budgets: {
      warn_turn_count: normalizeCount(budget.warn_turn_count),
      fail_turn_count: normalizeCount(budget.fail_turn_count),
      warn_input_tokens: normalizeCount(budget.warn_input_tokens),
      fail_input_tokens: normalizeCount(budget.fail_input_tokens),
    },
    warnings,
    failures,
  };
}

export function evaluateWpTokenBudget(ledger = {}) {
  const summary = ledger?.summary || {};
  const roleTotals = ledger?.role_totals || {};
  const totalEvaluation = evaluateBudgetSlice("TOTAL", summary, WP_TOTAL_TOKEN_BUDGET);
  const roles = {};
  let status = totalEvaluation.status;
  const warnings = [...totalEvaluation.warnings];
  const failures = [...totalEvaluation.failures];

  for (const [roleName, budget] of Object.entries(WP_ROLE_TOKEN_BUDGETS)) {
    const evaluation = evaluateBudgetSlice(roleName, roleTotals[roleName] || {}, budget);
    roles[roleName] = evaluation;
    status = worseStatus(status, evaluation.status);
    warnings.push(...evaluation.warnings);
    failures.push(...evaluation.failures);
  }

  const commandCount = normalizeCount(summary.command_count);
  const turnCount = normalizeCount(summary.turn_count);
  if (commandCount === 0 && turnCount === 0) {
    return {
      policy_id: WP_TOKEN_BUDGET_POLICY_ID,
      status: "NO_OUTPUTS",
      blocker_class: "NONE",
      invalidity_code: "",
      total: totalEvaluation,
      roles,
      warnings: [],
      failures: [],
      summary: "No settled WP session output was found, so token budget evaluation is not applicable yet.",
    };
  }

  return {
    policy_id: WP_TOKEN_BUDGET_POLICY_ID,
    status,
    blocker_class: status === "FAIL" ? "POLICY_CONFLICT" : "NONE",
    invalidity_code: status === "FAIL" ? "TOKEN_BUDGET_EXCEEDED" : "",
    total: totalEvaluation,
    roles,
    warnings,
    failures,
    summary:
      status === "FAIL"
        ? "WP turn/fresh-input spend exceeded the governed fail budget and requires lane repair before more orchestrator-managed work continues."
        : status === "WARN"
          ? "WP turn/fresh-input spend exceeded the governed warning budget and should be compacted before further ambiguity-driven retries."
          : warnings.length > 0
            ? "WP fresh-input spend is within the governed budget envelope, but gross cached replay has crossed telemetry thresholds and should be watched for compaction."
            : "WP fresh-input spend is within the governed budget envelope.",
  };
}
