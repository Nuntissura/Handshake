import assert from "node:assert/strict";
import test from "node:test";

import {
  buildManualRelayDispatchPrompt,
  deriveManualRelayEnvelope,
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
