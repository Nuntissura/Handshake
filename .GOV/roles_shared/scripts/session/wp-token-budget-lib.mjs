import {
  WP_ROLE_TOKEN_BUDGETS,
  WP_TOKEN_BUDGET_POLICY_ID,
  WP_TOTAL_TOKEN_BUDGET,
} from "./session-policy.mjs";
import { appendJsonlLine } from "./session-registry-lib.mjs";
import { governanceRuntimeAbsPath } from "../lib/runtime-paths.mjs";

export const ORCHESTRATOR_COST_GOVERNOR_POLICY_ID = "ORCHESTRATOR_MANAGED_COST_GOVERNOR_V1";

export const ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS = Object.freeze({
  warn_command_count: 18,
  fail_command_count: 28,
  warn_elapsed_hours: 8,
  fail_elapsed_hours: 14,
  warn_closeout_repair_attempts: 2,
  fail_closeout_repair_attempts: 3,
  warn_full_surface_rereads: 2,
  fail_full_surface_rereads: 3,
});

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
  const enforcementMode = "DIAGNOSTIC_ONLY";

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
      enforcement_mode: enforcementMode,
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
    enforcement_mode: enforcementMode,
    status,
    blocker_class: "NONE",
    invalidity_code: "",
    total: totalEvaluation,
    roles,
    warnings,
    failures,
    summary:
      status === "FAIL"
        ? "WP turn/fresh-input spend exceeded the governed high-cost threshold. Continue the WP, but record the overrun mechanically in the dossier and use compaction or context rotation to reduce further waste."
        : status === "WARN"
          ? "WP turn/fresh-input spend exceeded the governed warning threshold. Continue the WP and treat the overrun as diagnostic telemetry for compaction and workflow repair."
          : warnings.length > 0
            ? "WP fresh-input spend is within the governed envelope, but cached replay volume has crossed telemetry thresholds and should be watched for compaction."
            : "WP fresh-input spend is within the governed diagnostic envelope.",
  };
}

function metricBand(actual, warnLimit, failLimit) {
  const value = normalizeCount(actual);
  const warn = normalizeCount(warnLimit);
  const fail = normalizeCount(failLimit);
  if (fail > 0 && value > fail) return "FAIL";
  if (warn > 0 && value > warn) return "WARN";
  return "PASS";
}

function governorStateFromWorst(status) {
  switch (String(status || "").trim().toUpperCase()) {
    case "FAIL":
      return "RECOVERY_MODE";
    case "WARN":
      return "WARN";
    default:
      return "OK";
  }
}

function elapsedHoursFromLedger(ledger = {}) {
  const timestamps = (Array.isArray(ledger?.commands) ? ledger.commands : [])
    .map((entry) => Date.parse(String(entry?.processed_at || "")))
    .filter((value) => Number.isFinite(value))
    .sort((left, right) => left - right);
  if (timestamps.length < 2) return 0;
  return (timestamps.at(-1) - timestamps[0]) / (1000 * 60 * 60);
}

export function evaluateOrchestratorCostGovernor({
  ledger = {},
  commandTelemetry = {},
  tokenBudget = null,
} = {}) {
  const budget = tokenBudget || evaluateWpTokenBudget(ledger);
  const summary = ledger?.summary || {};
  const commandCount = normalizeCount(commandTelemetry.command_count ?? summary.command_count);
  const elapsedHours = Number(commandTelemetry.elapsed_hours ?? elapsedHoursFromLedger(ledger));
  const closeoutRepairAttempts = normalizeCount(commandTelemetry.closeout_repair_attempts);
  const fullSurfaceRereads = normalizeCount(commandTelemetry.full_surface_rereads);
  let worst = budget.status === "FAIL" ? "FAIL" : budget.status === "WARN" ? "WARN" : "PASS";
  const warnings = [...(budget.warnings || [])];
  const failures = [...(budget.failures || [])];

  const commandBand = metricBand(
    commandCount,
    ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.warn_command_count,
    ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.fail_command_count,
  );
  const elapsedBand = Number.isFinite(elapsedHours)
    ? metricBand(
        Math.trunc(elapsedHours),
        ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.warn_elapsed_hours,
        ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.fail_elapsed_hours,
      )
    : "PASS";
  const repairBand = metricBand(
    closeoutRepairAttempts,
    ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.warn_closeout_repair_attempts,
    ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.fail_closeout_repair_attempts,
  );
  const rereadBand = metricBand(
    fullSurfaceRereads,
    ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.warn_full_surface_rereads,
    ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.fail_full_surface_rereads,
  );

  for (const [label, band, value, thresholds] of [
    ["command_count", commandBand, commandCount, [ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.warn_command_count, ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.fail_command_count]],
    ["elapsed_hours", elapsedBand, elapsedHours, [ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.warn_elapsed_hours, ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.fail_elapsed_hours]],
    ["closeout_repair_attempts", repairBand, closeoutRepairAttempts, [ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.warn_closeout_repair_attempts, ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.fail_closeout_repair_attempts]],
    ["full_surface_rereads", rereadBand, fullSurfaceRereads, [ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.warn_full_surface_rereads, ORCHESTRATOR_COST_GOVERNOR_THRESHOLDS.fail_full_surface_rereads]],
  ]) {
    if (band === "FAIL") {
      failures.push(`${label} ${value} exceeded fail budget ${thresholds[1]}`);
      worst = worseStatus(worst, "FAIL");
    } else if (band === "WARN") {
      warnings.push(`${label} ${value} exceeded warn budget ${thresholds[0]}`);
      worst = worseStatus(worst, "WARN");
    }
  }

  const loopBreakerRequired = repairBand === "FAIL" || rereadBand === "FAIL";
  const state = loopBreakerRequired ? "OVERRIDE_REQUIRED" : governorStateFromWorst(worst);
  return {
    policy_id: ORCHESTRATOR_COST_GOVERNOR_POLICY_ID,
    state,
    token_budget_status: budget.status,
    command_count: commandCount,
    elapsed_hours: Number.isFinite(elapsedHours) ? elapsedHours : 0,
    closeout_repair_attempts: closeoutRepairAttempts,
    full_surface_rereads: fullSurfaceRereads,
    warnings,
    failures,
    permitted_commands: state === "OK"
      ? ["normal_orchestrator_next", "normal_orchestrator_steer"]
      : [
          "just wp-truth-bundle WP-{ID}",
          "just closeout-repair WP-{ID}",
          "just orchestrator-rescue WP-{ID}",
          "just orchestrator-next WP-{ID}",
          "operator-authorized override",
        ],
    summary:
      state === "OVERRIDE_REQUIRED"
        ? "Orchestrator recovery loop budget is exhausted; broad rediscovery or steering requires explicit Operator override."
        : state === "RECOVERY_MODE"
          ? "Orchestrator cost threshold crossed; use compact truth, loop breakers, or the projected next legal governed role action."
          : state === "WARN"
            ? "Orchestrator cost warning threshold crossed; prefer compact truth before broad rediscovery."
            : "Orchestrator cost governor is within budget.",
  };
}

export function costGovernorSteerDecision({
  governor = {},
  explicitTargetRole = "",
  nextActor = "",
  defaultNextActor = "",
  overrideReason = "",
} = {}) {
  const state = String(governor?.state || "OK").trim().toUpperCase();
  const targetRole = String(explicitTargetRole || "").trim().toUpperCase();
  const normalizedNextActor = String(nextActor || "").trim().toUpperCase();
  const normalizedDefaultNextActor = String(defaultNextActor || "").trim().toUpperCase();
  const reason = String(overrideReason || "").trim();
  const hasOverride = reason.length > 0;
  const projectedNextLegalAction = Boolean(
    normalizedNextActor
      && normalizedDefaultNextActor
      && normalizedNextActor === normalizedDefaultNextActor,
  );

  if (!["WARN", "RECOVERY_MODE", "OVERRIDE_REQUIRED"].includes(state)) {
    return {
      ok: true,
      state,
      override_required: false,
      override_recorded: false,
      projected_next_legal_action: projectedNextLegalAction,
      reason: "cost-governor-within-budget",
      failures: [],
    };
  }

  if (state === "WARN") {
    return {
      ok: true,
      state,
      override_required: false,
      override_recorded: false,
      projected_next_legal_action: projectedNextLegalAction,
      reason: "cost-governor-warning",
      failures: [],
    };
  }

  if (state === "OVERRIDE_REQUIRED" && !hasOverride) {
    return {
      ok: false,
      state,
      override_required: true,
      override_recorded: false,
      projected_next_legal_action: projectedNextLegalAction,
      reason: "operator-override-required",
      failures: [
        "Orchestrator cost governor requires explicit Operator override before another steer.",
        ...(governor?.failures || []),
      ],
    };
  }

  if (state === "RECOVERY_MODE" && targetRole && !projectedNextLegalAction && !hasOverride) {
    return {
      ok: false,
      state,
      override_required: true,
      override_recorded: false,
      projected_next_legal_action: projectedNextLegalAction,
      reason: "broad-steer-blocked-in-recovery-mode",
      failures: [
        "Recovery mode permits only compact truth, loop breakers, or the projected next legal governed role action without Operator override.",
        `target_role=${targetRole}`,
        `default_next_actor=${normalizedDefaultNextActor || "<missing>"}`,
        ...(governor?.failures || []),
      ],
    };
  }

  return {
    ok: true,
    state,
    override_required: hasOverride,
    override_recorded: hasOverride,
    projected_next_legal_action: projectedNextLegalAction,
    reason: hasOverride ? "operator-override-supplied" : "projected-next-legal-action",
    failures: [],
  };
}

export function recordOrchestratorCostOverride({
  wpId = "",
  reason = "",
  actor = "ORCHESTRATOR",
  timestamp = new Date().toISOString(),
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  const normalizedReason = String(reason || "").trim();
  if (!normalizedWpId) throw new Error("recordOrchestratorCostOverride requires wpId");
  if (normalizedReason.length < 20) throw new Error("Cost governor override reason must be at least 20 characters");
  const filePath = governanceRuntimeAbsPath("roles_shared", "WP_COMMUNICATIONS", normalizedWpId, "COST_GOVERNOR_OVERRIDES.jsonl");
  const entry = {
    schema_id: "hsk.orchestrator_cost_override@1",
    schema_version: "orchestrator_cost_override_v1",
    timestamp_utc: timestamp,
    wp_id: normalizedWpId,
    actor: String(actor || "ORCHESTRATOR").trim().toUpperCase(),
    reason: normalizedReason,
  };
  appendJsonlLine(filePath, entry);
  return { filePath, entry };
}
