import assert from "node:assert/strict";
import test from "node:test";

import { finalizeTerminalSessions } from "../scripts/session/terminal-verdict-session-finalizer-lib.mjs";

test("finalizeTerminalSessions classifies stale READY residue after terminal verdict", () => {
  const result = finalizeTerminalSessions({
    wpId: "WP-TEST",
    terminalRecord: { verdict: "PASS" },
    sessions: [
      { wp_id: "WP-TEST", role: "CODER", runtime_state: "READY" },
      { wp_id: "WP-TEST", role: "WP_VALIDATOR", runtime_state: "CLOSED" },
      { wp_id: "WP-OTHER", role: "CODER", runtime_state: "READY" },
    ],
  });

  assert.equal(result.status, "FINALIZE_READY");
  assert.equal(result.staleReadySessions.length, 1);
  assert.equal(result.terminalResidue[0].residue_kind, "STALE_READY_SESSION");
});

test("finalizeTerminalSessions reports active and queued blockers separately", () => {
  const result = finalizeTerminalSessions({
    wpId: "WP-TEST",
    terminalRecord: { verdict: "FAIL" },
    sessions: [
      { wp_id: "WP-TEST", role: "CODER", runtime_state: "COMMAND_RUNNING", pending_control_queue_count: 0 },
      { wp_id: "WP-TEST", role: "WP_VALIDATOR", runtime_state: "READY", pending_control_queue_count: 1 },
    ],
  });

  assert.equal(result.status, "BLOCKED");
  assert.equal(result.activeBlockers.length, 1);
  assert.equal(result.queuedBlockers.length, 1);
  assert.equal(result.staleReadySessions.length, 0);
});

test("finalizeTerminalSessions ignores cleanup before terminal verdict", () => {
  const result = finalizeTerminalSessions({
    wpId: "WP-TEST",
    terminalRecord: { verdict: "IN_PROGRESS" },
    sessions: [
      { wp_id: "WP-TEST", role: "CODER", runtime_state: "READY" },
    ],
  });

  assert.equal(result.status, "NOT_TERMINAL");
  assert.equal(result.terminal, false);
});
