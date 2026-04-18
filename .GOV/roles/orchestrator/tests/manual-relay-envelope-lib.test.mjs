import assert from "node:assert/strict";
import test from "node:test";

import {
  buildRelayDispatchPrompt,
  deriveRelayEnvelope,
  buildManualRelayDispatchPrompt,
  deriveManualRelayEnvelope,
  preferredTargetSession,
} from "../scripts/lib/manual-relay-envelope-lib.mjs";

test("deriveManualRelayEnvelope prefers targeted notifications and classifies relay kind", () => {
  const envelope = deriveManualRelayEnvelope({
    wpId: "WP-TEST-MANUAL-RELAY-LIB",
    runtimeStatus: {
      waiting_on: "WP_VALIDATOR_REVIEW",
      current_phase: "VALIDATION",
      open_review_items: [
        {
          correlation_id: "handoff-1",
          receipt_kind: "CODER_HANDOFF",
          summary: "Fallback review summary.",
          opened_by_role: "CODER",
          opened_by_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          requires_ack: true,
          opened_at: "2026-04-05T10:00:00Z",
          updated_at: "2026-04-05T10:00:00Z",
        },
      ],
    },
    nextActor: "WP_VALIDATOR",
    targetSession: "wpv-1",
    notifications: {
      notifications: [
        {
          timestamp_utc: "2026-04-05T10:01:00Z",
          source_kind: "CODER_HANDOFF",
          source_role: "CODER",
          source_session: "coder-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-1",
          correlation_id: "handoff-1",
          summary: "Ready for validator review.",
        },
      ],
    },
    dispatchAction: "SEND_PROMPT",
  });

  assert.equal(envelope.fromEndpoint, "CODER:coder-1");
  assert.equal(envelope.toEndpoint, "WP_VALIDATOR:wpv-1");
  assert.equal(envelope.relayKind, "HANDOFF");
  assert.equal(envelope.sourceKind, "CODER_HANDOFF");
  assert.equal(envelope.correlationId, "handoff-1");
  assert.equal(envelope.ackRequired, true);
  assert.equal(envelope.message, "Ready for validator review.");
  assert.match(envelope.operatorExplainer.join("\n"), /Dispatch action is SEND_PROMPT/);
});

test("buildManualRelayDispatchPrompt injects typed relay context for the target role", () => {
  const prompt = buildManualRelayDispatchPrompt({
    basePrompt: "RESUME GOVERNED WP_VALIDATOR lane for WP-TEST-MANUAL-RELAY-LIB.",
    envelope: {
      fromEndpoint: "CODER:coder-1",
      toEndpoint: "WP_VALIDATOR:wpv-1",
      relayKind: "QUESTION",
      sourceKind: "REVIEW_REQUEST",
      correlationId: "corr-22",
      ackRequired: true,
      message: "Please confirm whether MT-002 is clear to continue.",
    },
  });

  assert.match(prompt, /MANUAL_RELAY_CONTEXT \[CX-MANUAL-RELAY-004\]/);
  assert.match(prompt, /DIRECT_ROLE_MESSAGE \[CX-MANUAL-RELAY-005\]/);
  assert.match(prompt, /RELAY_KIND: QUESTION/);
  assert.match(prompt, /SOURCE_KIND: REVIEW_REQUEST/);
  assert.match(prompt, /Please confirm whether MT-002 is clear to continue\./);
  assert.doesNotMatch(prompt, /OPERATOR_EXPLAINER/);
});

test("deriveRelayEnvelope falls back to runtime waiting state when no targeted notification exists", () => {
  const envelope = deriveRelayEnvelope({
    wpId: "WP-TEST-GOVERNED-ROUTE",
    runtimeStatus: {
      waiting_on: "CODER_INTENT",
      current_phase: "BOOTSTRAP",
    },
    nextActor: "CODER",
    targetSession: "coder-1",
    notifications: { notifications: [] },
    dispatchAction: "START_SESSION",
  });

  assert.equal(envelope.fromEndpoint, "RUNTIME");
  assert.equal(envelope.toEndpoint, "CODER:coder-1");
  assert.equal(envelope.relayKind, "INTENT");
  assert.equal(envelope.sourceKind, "CODER_INTENT");
  assert.match(envelope.message, /Runtime is waiting on CODER_INTENT/);
});

test("deriveRelayEnvelope reuses route anchor context when no targeted notification or review item exists", () => {
  const runtimeStatus = {
    waiting_on: "OPEN_REVIEW_ITEM_REVIEW_REQUEST",
    current_phase: "VALIDATION",
    route_anchor_kind: "REVIEW_REQUEST",
    route_anchor_correlation_id: "review-2",
    route_anchor_target_session: "wpv-2",
    open_review_items: [],
  };
  const targetSession = preferredTargetSession(runtimeStatus, null);
  const envelope = deriveRelayEnvelope({
    wpId: "WP-TEST-GOVERNED-ROUTE",
    runtimeStatus,
    nextActor: "WP_VALIDATOR",
    targetSession,
    notifications: { notifications: [] },
    dispatchAction: "SEND_PROMPT",
  });

  assert.equal(targetSession, "wpv-2");
  assert.equal(envelope.toEndpoint, "WP_VALIDATOR:wpv-2");
  assert.equal(envelope.sourceKind, "REVIEW_REQUEST");
  assert.equal(envelope.correlationId, "review-2");
  assert.match(envelope.message, /OPEN_REVIEW_ITEM_REVIEW_REQUEST/);
});

test("buildRelayDispatchPrompt supports orchestrator-managed route labels", () => {
  const prompt = buildRelayDispatchPrompt({
    basePrompt: "RESUME GOVERNED CODER lane for WP-TEST-GOVERNED-ROUTE.",
    envelope: {
      fromEndpoint: "WP_VALIDATOR:wpv-1",
      toEndpoint: "CODER:coder-1",
      relayKind: "QUESTION",
      sourceKind: "VALIDATOR_QUERY",
      correlationId: "kick-22",
      ackRequired: true,
      message: "Clarify whether the next step stays inside MT-002.",
    },
    contextLabel: "GOVERNED_ROUTE_CONTEXT [CX-ROUTE-001]",
    messageLabel: "DIRECT_ROLE_MESSAGE [CX-ROUTE-002]",
    terminalInstructions: [
      "Treat DIRECT_ROLE_MESSAGE as the current receipt/notification-derived payload for WORKFLOW_LANE=ORCHESTRATOR_MANAGED.",
    ],
  });

  assert.match(prompt, /GOVERNED_ROUTE_CONTEXT \[CX-ROUTE-001\]/);
  assert.match(prompt, /DIRECT_ROLE_MESSAGE \[CX-ROUTE-002\]/);
  assert.match(prompt, /FROM: WP_VALIDATOR:wpv-1/);
  assert.match(prompt, /TO: CODER:coder-1/);
  assert.match(prompt, /RELAY_KIND: QUESTION/);
  assert.match(prompt, /SOURCE_KIND: VALIDATOR_QUERY/);
  assert.match(prompt, /Clarify whether the next step stays inside MT-002\./);
  assert.match(prompt, /WORKFLOW_LANE=ORCHESTRATOR_MANAGED/);
});
