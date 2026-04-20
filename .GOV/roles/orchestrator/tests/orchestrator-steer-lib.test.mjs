import assert from "node:assert/strict";
import test from "node:test";

import {
  nextQueuedControlRequest,
  pendingControlQueueCount,
  steerActionForSession,
} from "../scripts/lib/orchestrator-steer-lib.mjs";

test("orchestrator-steer reuses an existing thread even after a failed command", () => {
  assert.equal(
    steerActionForSession({
      session_thread_id: "019d46d9-161b-7ed2-a626-aa6a9c7c8e8d",
      runtime_state: "FAILED",
    }),
    "SEND_PROMPT",
  );
});

test("orchestrator-steer starts a new session when no steerable thread exists", () => {
  assert.equal(
    steerActionForSession({
      session_thread_id: "",
      runtime_state: "FAILED",
    }),
    "START_SESSION",
  );
  assert.equal(
    steerActionForSession({
      session_thread_id: "019d46d9-161b-7ed2-a626-aa6a9c7c8e8d",
      runtime_state: "CLOSED",
    }),
    "START_SESSION",
  );
});

test("orchestrator-steer queue helpers read session summary fields", () => {
  const session = {
    pending_control_queue_count: 2,
    next_queued_control_request: {
      command_kind: "SEND_PROMPT",
      queued_at: "2026-04-20T10:11:12.000Z",
      summary: "Resume queued follow-up",
    },
  };

  assert.equal(pendingControlQueueCount(session), 2);
  assert.deepEqual(nextQueuedControlRequest(session), session.next_queued_control_request);
});

test("orchestrator-steer queue helpers fall back to raw pending queue entries", () => {
  const session = {
    pending_control_queue: [
      {
        command_kind: "SEND_PROMPT",
        queued_at: "2026-04-20T10:11:12.000Z",
        summary: "First queued follow-up",
      },
      {
        command_kind: "SEND_PROMPT",
        queued_at: "2026-04-20T10:12:12.000Z",
        summary: "Second queued follow-up",
      },
    ],
  };

  assert.equal(pendingControlQueueCount(session), 2);
  assert.deepEqual(nextQueuedControlRequest(session), session.pending_control_queue[0]);
});
