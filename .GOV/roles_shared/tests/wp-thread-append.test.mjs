import assert from "node:assert/strict";
import test from "node:test";

import { parseThreadAppendCliArgs } from "../scripts/wp/wp-thread-append.mjs";

test("parseThreadAppendCliArgs maps stable wrapper metadata", () => {
  const parsed = parseThreadAppendCliArgs([
    "WP-TEST-v1",
    "ORCHESTRATOR",
    "orchestrator:test",
    "Message",
    "target=CODER",
    "target_role=CODER",
    "target_session=coder:test",
    "correlation_id=kickoff-1",
    "requires_ack=true",
    "ack_for=kickoff-1",
    "spec_anchor=Spec v1",
    "packet_row_ref=MT-001",
  ]);

  assert.equal(parsed.target, "CODER");
  assert.equal(parsed.targetRole, "CODER");
  assert.equal(parsed.targetSession, "coder:test");
  assert.equal(parsed.correlationId, "kickoff-1");
  assert.equal(parsed.requiresAck, "true");
  assert.equal(parsed.ackFor, "kickoff-1");
  assert.equal(parsed.specAnchor, "Spec v1");
  assert.equal(parsed.packetRowRef, "MT-001");
});

test("parseThreadAppendCliArgs unwraps user-authored name=value passed through Just positions", () => {
  const parsed = parseThreadAppendCliArgs([
    "WP-TEST-v1",
    "ORCHESTRATOR",
    "orchestrator:test",
    "Message",
    "target=target_role=CODER",
    "target_role=target_session=coder:test",
    "target_session=correlation_id=kickoff-1",
    "correlation_id=requires_ack=true",
    "requires_ack=ack_for=kickoff-1",
    "ack_for=spec_anchor=Spec v1",
    "spec_anchor=packet_row_ref=MT-001",
  ]);

  assert.equal(parsed.target, undefined);
  assert.equal(parsed.targetRole, "CODER");
  assert.equal(parsed.targetSession, "coder:test");
  assert.equal(parsed.correlationId, "kickoff-1");
  assert.equal(parsed.requiresAck, "true");
  assert.equal(parsed.ackFor, "kickoff-1");
  assert.equal(parsed.specAnchor, "Spec v1");
  assert.equal(parsed.packetRowRef, "MT-001");
});
