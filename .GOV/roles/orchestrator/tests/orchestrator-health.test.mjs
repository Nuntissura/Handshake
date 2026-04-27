import assert from "node:assert/strict";
import test from "node:test";

import {
  formatDuration,
  latestSessionActivityIso,
  nextSafeCommand,
  sessionHealthLine,
} from "../scripts/orchestrator-health-lib.mjs";

test("formatDuration keeps health ages compact", () => {
  assert.equal(formatDuration(null), "<unknown>");
  assert.equal(formatDuration(42), "42s");
  assert.equal(formatDuration(600), "10m");
  assert.equal(formatDuration(7500), "2h5m");
});

test("latestSessionActivityIso prefers governed action and queue timestamps", () => {
  assert.equal(latestSessionActivityIso({
    updated_at: "2026-04-26T10:00:00.000Z",
    effective_governed_action: { updated_at: "2026-04-26T10:05:00.000Z" },
    next_queued_control_request: { queued_at: "2026-04-26T10:10:00.000Z" },
  }), "2026-04-26T10:10:00.000Z");
});

test("sessionHealthLine includes model profile thread command queue and stale age", () => {
  const line = sessionHealthLine({
    role: "CODER",
    runtime_state: "READY",
    requested_profile_id: "gpt55-xhigh",
    session_thread_id: "thread-1",
    effective_governed_action: {
      command_kind: "SEND_PROMPT",
      status: "COMPLETED",
      outcome_state: "READY",
      updated_at: "2026-04-26T10:00:00.000Z",
    },
    pending_control_queue_count: 2,
    local_worktree_dir: "../wtc-example",
  }, {
    requested_model: "gpt-5.5",
  }, new Date("2026-04-26T10:12:00.000Z"));

  assert.match(line, /role=CODER/);
  assert.match(line, /model=gpt-5\.5/);
  assert.match(line, /profile=gpt55-xhigh/);
  assert.match(line, /thread=thread-1/);
  assert.match(line, /command=SEND_PROMPT\/COMPLETED/);
  assert.match(line, /queued=2/);
  assert.match(line, /stale=12m/);
});

test("nextSafeCommand remains a read-only orchestrator-next route", () => {
  assert.equal(nextSafeCommand({ wpId: "WP-TEST-v1" }), "just orchestrator-next WP-TEST-v1");
  assert.equal(nextSafeCommand({}), "just orchestrator-next");
});
