import assert from "node:assert/strict";
import test from "node:test";
import {
  normalizeRelayEscalationPolicy,
  relayEscalationPolicyBudgetLabel,
  relayEscalationPolicyInlineSummary,
  relayEscalationPolicyStrategyLabel,
} from "../scripts/lib/wp-relay-policy-lib.mjs";

test("normalizeRelayEscalationPolicy sanitizes relay policy fields", () => {
  const normalized = normalizeRelayEscalationPolicy({
    source_surface: " watchdog ",
    failure_class: " DUPLICATE_REWAKE_LOOP ",
    policy_state: " AUTO_RETRY_BLOCKED ",
    next_strategy: " ALTERNATE_METHOD ",
    reason_code: " same-failure ",
    budget_scope: " SAME_FAILURE_REWAKE ",
    budget_used: "2",
    budget_limit: "2",
    summary: " repeated wake failures ",
    updated_at: " 2099-01-01T10:00:00Z ",
  });

  assert.deepEqual(normalized, {
    source_surface: "watchdog",
    failure_class: "DUPLICATE_REWAKE_LOOP",
    policy_state: "AUTO_RETRY_BLOCKED",
    next_strategy: "ALTERNATE_METHOD",
    reason_code: "same-failure",
    budget_scope: "SAME_FAILURE_REWAKE",
    budget_used: 2,
    budget_limit: 2,
    summary: "repeated wake failures",
    updated_at: "2099-01-01T10:00:00Z",
  });
});

test("relayEscalationPolicyBudgetLabel and inline helpers format policy consistently", () => {
  const policy = {
    source_surface: "RELAY_WATCHDOG",
    failure_class: "WORKER_INTERRUPT_LIMIT",
    policy_state: "AUTO_RETRY_BLOCKED",
    next_strategy: "ALTERNATE_MODEL",
    reason_code: "MAX_WORKER_INTERRUPT_CYCLES_REACHED",
    budget_scope: "WORKER_INTERRUPT_CYCLE",
    budget_used: 3,
    budget_limit: 3,
    summary: "Interrupt budget exhausted.",
    updated_at: "2099-01-01T10:01:00Z",
  };

  assert.equal(relayEscalationPolicyBudgetLabel(policy), "WORKER_INTERRUPT_CYCLE:3/3");
  assert.equal(relayEscalationPolicyStrategyLabel(policy), "alternate-model");
  assert.match(
    relayEscalationPolicyInlineSummary(policy),
    /failure_class=WORKER_INTERRUPT_LIMIT \| policy=AUTO_RETRY_BLOCKED->ALTERNATE_MODEL \| budget=WORKER_INTERRUPT_CYCLE:3\/3/,
  );
});

test("relayEscalationPolicyBudgetLabel returns none for missing or NONE-scoped policy", () => {
  assert.equal(relayEscalationPolicyBudgetLabel(null), "none");
  assert.equal(relayEscalationPolicyBudgetLabel({
    source_surface: "RELAY_WATCHDOG",
    failure_class: "ACTIVE_RUN_PRESENT",
    policy_state: "DEFERRED",
    next_strategy: "QUEUED_DEFER",
    reason_code: "ACTIVE_RUN_PRESENT",
    budget_scope: "NONE",
    budget_used: 0,
    budget_limit: 0,
    summary: "Deferred while a run is active.",
    updated_at: "2099-01-01T10:02:00Z",
  }), "none");
});

