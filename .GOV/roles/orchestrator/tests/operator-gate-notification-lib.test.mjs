import assert from "node:assert/strict";
import test from "node:test";

import {
  buildOperatorGateNotificationCandidate,
  parseOrchestratorLifecycleOutput,
} from "../scripts/lib/operator-gate-notification-lib.mjs";

test("parseOrchestratorLifecycleOutput extracts approval gate fields", () => {
  const parsed = parseOrchestratorLifecycleOutput(`
LIFECYCLE [CX-LIFE-001]
- WP_ID: WP-1-Distillation-v2
- STAGE: APPROVAL
- NEXT: SIGNATURE

OPERATOR_ACTION: Collect explicit approval + one-time signature bundle for WP-1-Distillation-v2 (signature + workflow lane + execution owner)

BLOCKER_CLASS: PRE_SIGNATURE_APPROVAL_REQUIRED

CONFIDENCE: HIGH (explicit)

STATE: Refinement recorded; signature not yet recorded.

NEXT_COMMANDS [CX-GATE-UX-001]
- # Paste the FULL Technical Refinement Block from .GOV/refinements/WP-1-Distillation-v2.md in chat (verbatim; no summary).
- just record-signature WP-1-Distillation-v2 {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} {Coder-A..Coder-Z}
`);

  assert.equal(parsed.stage, "APPROVAL");
  assert.equal(parsed.next, "SIGNATURE");
  assert.equal(parsed.operatorAction, "Collect explicit approval + one-time signature bundle for WP-1-Distillation-v2 (signature + workflow lane + execution owner)");
  assert.equal(parsed.blockerClass, "PRE_SIGNATURE_APPROVAL_REQUIRED");
  assert.equal(parsed.state, "Refinement recorded; signature not yet recorded.");
  assert.equal(parsed.nextCommands.length, 2);
});

test("buildOperatorGateNotificationCandidate creates durable operator gate correlation", () => {
  const candidate = buildOperatorGateNotificationCandidate({
    wpId: "WP-1-Distillation-v2",
    lifecycle: {
      stage: "APPROVAL",
      next: "SIGNATURE",
      operatorAction: "Collect explicit approval + one-time signature bundle for WP-1-Distillation-v2 (signature + workflow lane + execution owner)",
      blockerClass: "PRE_SIGNATURE_APPROVAL_REQUIRED",
      state: "Refinement recorded; signature not yet recorded.",
    },
  });

  assert.equal(candidate.sourceKind, "OPERATOR_GATE");
  assert.equal(candidate.targetRole, "OPERATOR");
  assert.equal(candidate.correlationId, "operator-gate:WP-1-Distillation-v2:PRE_SIGNATURE_APPROVAL_REQUIRED:APPROVAL:SIGNATURE");
  assert.match(candidate.summary, /OPERATOR_GATE: APPROVAL -> SIGNATURE/);
  assert.match(candidate.summary, /PRE_SIGNATURE_APPROVAL_REQUIRED/);
  assert.match(candidate.summary, /Refinement recorded; signature not yet recorded\./);
});

test("buildOperatorGateNotificationCandidate ignores non-operator lifecycle states", () => {
  const candidate = buildOperatorGateNotificationCandidate({
    wpId: "WP-1-Example-v1",
    lifecycle: {
      stage: "DELEGATION",
      next: "STOP",
      operatorAction: "NONE",
      blockerClass: "NONE",
      state: "No immediate governed action is pending.",
    },
  });

  assert.equal(candidate, null);
});
