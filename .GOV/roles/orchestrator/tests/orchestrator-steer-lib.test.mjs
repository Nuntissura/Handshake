import assert from "node:assert/strict";
import test from "node:test";

import { steerActionForSession } from "../scripts/lib/orchestrator-steer-lib.mjs";

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
