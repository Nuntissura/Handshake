import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  appendSessionEvent,
  getPredecessorSummary,
  sessionEventsFile,
} from "../scripts/session/predecessor-lookup-lib.mjs";

function makeRuntimeRoot() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "hsk-predecessor-"));
}

function cleanup(root) {
  fs.rmSync(root, { recursive: true, force: true });
}

function wordCount(text = "") {
  return String(text || "").trim().split(/\s+/).filter(Boolean).length;
}

function appendFixtureEvent(runtimeRootAbs, { wpId, role, sessionId, eventType, event }) {
  return appendSessionEvent({
    wpId,
    role,
    sessionId,
    eventType,
    event,
    runtimeRootAbs,
  });
}

test("getPredecessorSummary picks latest same-role session and applies event caps", async () => {
  const runtimeRootAbs = makeRuntimeRoot();
  try {
    const wpId = "WP-TEST-PREDECESSOR-v1";
    const role = "CODER";
    const older = {
      session_key: "CODER:older",
      session_id: "coder-older",
      wp_id: wpId,
      role,
      last_command_completed_at: "2026-04-27T10:00:00Z",
    };
    const recent = {
      session_key: "CODER:recent",
      session_id: "coder-recent",
      wp_id: wpId,
      role,
      last_command_completed_at: "2026-04-27T11:00:00Z",
    };
    const validator = {
      session_key: "WP_VALIDATOR:recent",
      session_id: "validator-recent",
      wp_id: wpId,
      role: "WP_VALIDATOR",
      last_command_completed_at: "2026-04-27T12:00:00Z",
    };
    const current = {
      session_key: "CODER:current",
      session_id: "coder-current",
      wp_id: wpId,
      role,
      last_event_at: "2026-04-27T12:30:00Z",
    };
    const registry = { sessions: [older, recent, validator, current] };

    appendFixtureEvent(runtimeRootAbs, {
      wpId,
      role,
      sessionId: older.session_id,
      eventType: "tool_call",
      event: { tool_name: "older_tool", result_class: "OK" },
    });
    appendFixtureEvent(runtimeRootAbs, {
      wpId,
      role: "WP_VALIDATOR",
      sessionId: validator.session_id,
      eventType: "tool_call",
      event: { tool_name: "validator_tool", result_class: "OK" },
    });

    for (let index = 1; index <= 12; index += 1) {
      appendFixtureEvent(runtimeRootAbs, {
        wpId,
        role,
        sessionId: recent.session_id,
        eventType: "tool_call",
        event: { tool_name: `tool_${String(index).padStart(2, "0")}`, result_class: "OK", args_summary: `arg ${index}` },
      });
    }
    for (let index = 1; index <= 6; index += 1) {
      appendFixtureEvent(runtimeRootAbs, {
        wpId,
        role,
        sessionId: recent.session_id,
        eventType: "receipt_emitted",
        event: { receipt_kind: `RECEIPT_${index}`, verb: "REPLY", mt_id: "MT-001" },
      });
    }
    for (let index = 1; index <= 4; index += 1) {
      appendFixtureEvent(runtimeRootAbs, {
        wpId,
        role,
        sessionId: recent.session_id,
        eventType: "file_touched",
        event: { path: `src/file_${index}.rs`, action: "edit" },
      });
    }
    for (let index = 1; index <= 4; index += 1) {
      appendFixtureEvent(runtimeRootAbs, {
        wpId,
        role,
        sessionId: recent.session_id,
        eventType: "mt_progression",
        event: { mt_id: "MT-001", transition: `state_${index}` },
      });
    }
    for (let index = 1; index <= 3; index += 1) {
      appendFixtureEvent(runtimeRootAbs, {
        wpId,
        role,
        sessionId: recent.session_id,
        eventType: "verdict_transition",
        event: { kind: "MT_VERDICT", mt_id: "MT-001", from: `FROM_${index}`, to: `TO_${index}` },
      });
    }
    appendFixtureEvent(runtimeRootAbs, {
      wpId,
      role,
      sessionId: recent.session_id,
      eventType: "steer_received",
      event: { source_role: "ORCHESTRATOR", summary: "resume the active MT" },
    });

    const summary = await getPredecessorSummary({
      wpId,
      role,
      currentSessionId: current.session_id,
      registry,
      runtimeRootAbs,
      tokenBudget: 500,
    });

    assert.ok(summary);
    assert.match(summary, /<predecessor-summary/);
    assert.match(summary, /PREDECESSOR_SESSION_ID: coder-recent/);
    assert.match(summary, /tool_03/);
    assert.match(summary, /tool_12/);
    assert.doesNotMatch(summary, /tool_01/);
    assert.doesNotMatch(summary, /older_tool/);
    assert.doesNotMatch(summary, /validator_tool/);
    assert.match(summary, /RECEIPT_2/);
    assert.match(summary, /RECEIPT_6/);
    assert.doesNotMatch(summary, /RECEIPT_1/);
    assert.match(summary, /src\/file_2\.rs/);
    assert.match(summary, /src\/file_4\.rs/);
    assert.doesNotMatch(summary, /src\/file_1\.rs/);
    assert.match(summary, /FROM_2->TO_2/);
    assert.match(summary, /FROM_3->TO_3/);
    assert.doesNotMatch(summary, /FROM_1->TO_1/);
    assert.match(summary, /resume the active MT/);
    assert.ok(wordCount(summary) <= 500);
  } finally {
    cleanup(runtimeRootAbs);
  }
});

test("getPredecessorSummary omits gracefully when no predecessor event log exists", async () => {
  const runtimeRootAbs = makeRuntimeRoot();
  try {
    const summary = await getPredecessorSummary({
      wpId: "WP-TEST-FIRST-SESSION-v1",
      role: "CODER",
      currentSessionId: "CODER:WP-TEST-FIRST-SESSION-v1",
      registry: {
        sessions: [
          {
            session_key: "CODER:WP-TEST-FIRST-SESSION-v1",
            session_id: "coder-current",
            wp_id: "WP-TEST-FIRST-SESSION-v1",
            role: "CODER",
            last_event_at: "2026-04-27T12:00:00Z",
          },
        ],
      },
      runtimeRootAbs,
    });

    assert.equal(summary, null);
  } finally {
    cleanup(runtimeRootAbs);
  }
});

test("getPredecessorSummary can treat the current session as predecessor during PreCompact", async () => {
  const runtimeRootAbs = makeRuntimeRoot();
  try {
    const wpId = "WP-TEST-PRECOMPACT-v1";
    const sessionId = "CODER:WP-TEST-PRECOMPACT-v1";
    const registry = {
      sessions: [
        {
          session_key: sessionId,
          session_id: sessionId,
          wp_id: wpId,
          role: "CODER",
          last_event_at: "2026-04-27T12:00:00Z",
        },
      ],
    };
    appendSessionEvent({
      wpId,
      role: "CODER",
      sessionId,
      eventType: "steer_received",
      event: { source_role: "ORCHESTRATOR", summary: "compact with MT-002 state preserved" },
      runtimeRootAbs,
    });

    const expectedPath = sessionEventsFile({ wpId, sessionId, runtimeRootAbs });
    assert.ok(fs.existsSync(expectedPath));

    const summary = await getPredecessorSummary({
      wpId,
      role: "CODER",
      currentSessionId: sessionId,
      includeCurrent: true,
      registry,
      runtimeRootAbs,
    });

    assert.ok(summary);
    assert.match(summary, /PREDECESSOR_SESSION_ID: CODER:WP-TEST-PRECOMPACT-v1/);
    assert.match(summary, /compact with MT-002 state preserved/);
  } finally {
    cleanup(runtimeRootAbs);
  }
});
