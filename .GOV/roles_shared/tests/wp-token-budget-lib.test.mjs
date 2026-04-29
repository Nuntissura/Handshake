import assert from "node:assert/strict";
import test from "node:test";
import {
  costGovernorSteerDecision,
  evaluateOrchestratorCostGovernor,
  evaluateWpTokenBudget,
} from "../scripts/session/wp-token-budget-lib.mjs";

function ledgerWith(summary, roleTotals = {}) {
  return {
    summary: {
      command_count: summary.command_count ?? 0,
      turn_count: summary.turn_count ?? 0,
      usage_totals: {
        input_tokens: summary.input_tokens ?? 0,
        cached_input_tokens: summary.cached_input_tokens ?? 0,
        output_tokens: summary.output_tokens ?? 0,
      },
    },
    role_totals: roleTotals,
  };
}

test("evaluateWpTokenBudget reports NO_OUTPUTS when no WP usage exists yet", () => {
  const budget = evaluateWpTokenBudget(ledgerWith({}));
  assert.equal(budget.status, "NO_OUTPUTS");
  assert.equal(budget.blocker_class, "NONE");
});

test("evaluateWpTokenBudget reports WARN when turn or token spend crosses warning thresholds", () => {
  const budget = evaluateWpTokenBudget(ledgerWith(
    { command_count: 8, turn_count: 25, input_tokens: 181000000 },
    {
      CODER: {
        command_count: 6,
        turn_count: 11,
        usage_totals: { input_tokens: 121000000, cached_input_tokens: 0, output_tokens: 0 },
      },
      WP_VALIDATOR: {
        command_count: 2,
        turn_count: 7,
        usage_totals: { input_tokens: 59000000, cached_input_tokens: 0, output_tokens: 0 },
      },
    },
  ));

  assert.equal(budget.status, "WARN");
  assert.equal(budget.blocker_class, "NONE");
  assert.match(budget.summary, /warning threshold/i);
  assert.equal(budget.roles.CODER.status, "WARN");
  assert.equal(budget.total.status, "WARN");
  assert.equal(budget.roles.CODER.fresh_input_tokens, 121000000);
});

test("evaluateWpTokenBudget reports FAIL diagnostically without a policy blocker when fail thresholds are exceeded", () => {
  const budget = evaluateWpTokenBudget(ledgerWith(
    { command_count: 12, turn_count: 33, input_tokens: 394494138 },
    {
      CODER: {
        command_count: 6,
        turn_count: 15,
        usage_totals: { input_tokens: 307679864, cached_input_tokens: 0, output_tokens: 0 },
      },
      WP_VALIDATOR: {
        command_count: 4,
        turn_count: 14,
        usage_totals: { input_tokens: 73570193, cached_input_tokens: 0, output_tokens: 0 },
      },
      INTEGRATION_VALIDATOR: {
        command_count: 2,
        turn_count: 4,
        usage_totals: { input_tokens: 13244081, cached_input_tokens: 0, output_tokens: 0 },
      },
    },
  ));

  assert.equal(budget.status, "FAIL");
  assert.equal(budget.enforcement_mode, "DIAGNOSTIC_ONLY");
  assert.equal(budget.blocker_class, "NONE");
  assert.equal(budget.invalidity_code, "");
  assert.match(budget.failures.join("\n"), /TOTAL fresh_input_tokens|CODER fresh_input_tokens/i);
  assert.match(budget.summary, /record the overrun mechanically/i);
});

test("evaluateWpTokenBudget keeps cached-heavy replay visible without blocking the lane", () => {
  const budget = evaluateWpTokenBudget(ledgerWith(
    {
      command_count: 19,
      turn_count: 11,
      input_tokens: 234332575,
      cached_input_tokens: 227941760,
    },
    {
      CODER: {
        command_count: 5,
        turn_count: 5,
        usage_totals: {
          input_tokens: 210118771,
          cached_input_tokens: 204985216,
          output_tokens: 654820,
        },
      },
      WP_VALIDATOR: {
        command_count: 5,
        turn_count: 0,
        usage_totals: {
          input_tokens: 0,
          cached_input_tokens: 0,
          output_tokens: 0,
        },
      },
    },
  ));

  assert.equal(budget.status, "PASS");
  assert.equal(budget.blocker_class, "NONE");
  assert.equal(budget.invalidity_code, "");
  assert.equal(budget.roles.CODER.fresh_input_tokens, 5133555);
  assert.equal(budget.total.fresh_input_tokens, 6390815);
  assert.match(budget.summary, /cached replay/i);
  assert.match(budget.warnings.join("\n"), /gross_input_tokens/i);
});

test("evaluateOrchestratorCostGovernor enters recovery mode on command budget failure", () => {
  const governor = evaluateOrchestratorCostGovernor({
    ledger: ledgerWith({ command_count: 31, turn_count: 2, input_tokens: 1000 }),
  });

  assert.equal(governor.state, "RECOVERY_MODE");
  assert.match(governor.failures.join("\n"), /command_count 31 exceeded fail budget/);
  assert.ok(governor.permitted_commands.includes("just wp-truth-bundle WP-{ID}"));
});

test("costGovernorSteerDecision permits projected recovery steer but blocks broad target steer", () => {
  const governor = evaluateOrchestratorCostGovernor({
    ledger: ledgerWith({ command_count: 31, turn_count: 2, input_tokens: 1000 }),
  });

  const projected = costGovernorSteerDecision({
    governor,
    nextActor: "CODER",
    defaultNextActor: "CODER",
  });
  assert.equal(projected.ok, true);
  assert.equal(projected.reason, "projected-next-legal-action");

  const broad = costGovernorSteerDecision({
    governor,
    explicitTargetRole: "WP_VALIDATOR",
    nextActor: "WP_VALIDATOR",
    defaultNextActor: "CODER",
  });
  assert.equal(broad.ok, false);
  assert.equal(broad.reason, "broad-steer-blocked-in-recovery-mode");
});

test("costGovernorSteerDecision requires override after repeated recovery loop attempts", () => {
  const governor = evaluateOrchestratorCostGovernor({
    ledger: ledgerWith({ command_count: 4, turn_count: 2, input_tokens: 1000 }),
    commandTelemetry: { full_surface_rereads: 4 },
  });

  assert.equal(governor.state, "OVERRIDE_REQUIRED");
  const blocked = costGovernorSteerDecision({
    governor,
    nextActor: "CODER",
    defaultNextActor: "CODER",
  });
  assert.equal(blocked.ok, false);
  assert.equal(blocked.reason, "operator-override-required");

  const overridden = costGovernorSteerDecision({
    governor,
    nextActor: "CODER",
    defaultNextActor: "CODER",
    overrideReason: "Operator reviewed the loop budget and authorized one final targeted recovery action.",
  });
  assert.equal(overridden.ok, true);
  assert.equal(overridden.override_recorded, true);
});
