import assert from "node:assert/strict";
import test from "node:test";

import {
  collectRecentRepomemCoverageWpIds,
  evaluateWpRepomemCoverage,
} from "../scripts/memory/repomem-coverage-lib.mjs";

test("repomem coverage passes when one role has open close and a WP durable checkpoint in the same session", () => {
  const coverage = evaluateWpRepomemCoverage({
    wpId: "WP-TEST-COVERAGE-v1",
    threadEntries: [
      {
        actorRole: "CODER",
      },
    ],
    conversationEntries: [
      {
        session_id: "CODER-20260422-090000",
        role: "CODER",
        checkpoint_type: "SESSION_OPEN",
        wp_id: "",
        topic: "role session open",
        content: "This session is about implementing deterministic coverage proof for the active coder lane.",
      },
      {
        session_id: "CODER-20260422-090000",
        role: "CODER",
        checkpoint_type: "INSIGHT",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "wp durable checkpoint",
        content: "The coder identified the exact proof shape needed to make repomem coverage attributable to this WP.",
      },
      {
        session_id: "CODER-20260422-090000",
        role: "CODER",
        checkpoint_type: "SESSION_CLOSE",
        wp_id: "",
        topic: "role session close",
        content: "The coder session closed after recording the WP-scoped durable checkpoint and confirming the implementation.",
      },
    ],
  });

  assert.equal(coverage.state, "PASS");
  assert.deepEqual(coverage.active_roles, ["CODER"]);
  assert.deepEqual(coverage.debt_roles, []);
  assert.deepEqual(coverage.debt_keys, []);
  assert.equal(coverage.role_details[0].status, "PASS");
});

test("repomem coverage reports missing proof when a materially active role has no conversation checkpoints", () => {
  const coverage = evaluateWpRepomemCoverage({
    wpId: "WP-TEST-COVERAGE-v1",
    receipts: [
      {
        wp_id: "WP-TEST-COVERAGE-v1",
        actor_role: "ORCHESTRATOR",
      },
    ],
    conversationEntries: [],
  });

  assert.equal(coverage.state, "DEBT");
  assert.deepEqual(coverage.active_roles, ["ORCHESTRATOR"]);
  assert.deepEqual(coverage.debt_roles, ["ORCHESTRATOR"]);
  assert.deepEqual(coverage.debt_keys, [
    "ORCHESTRATOR:NO_SESSION_OPEN",
    "ORCHESTRATOR:NO_SESSION_CLOSE",
    "ORCHESTRATOR:NO_WP_DURABLE_CHECKPOINT",
  ]);
});

test("repomem coverage does not treat auto-closed sessions as explicit close proof", () => {
  const coverage = evaluateWpRepomemCoverage({
    wpId: "WP-TEST-COVERAGE-v1",
    sessions: [
      {
        wp_id: "WP-TEST-COVERAGE-v1",
        role: "WP_VALIDATOR",
      },
    ],
    conversationEntries: [
      {
        session_id: "WP_VALIDATOR-20260422-100000",
        role: "WP_VALIDATOR",
        checkpoint_type: "SESSION_OPEN",
        wp_id: "",
        topic: "validator session open",
        content: "This validator session is about reviewing the packet evidence and closeout debt for the target WP.",
      },
      {
        session_id: "WP_VALIDATOR-20260422-100000",
        role: "WP_VALIDATOR",
        checkpoint_type: "DECISION",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "validator durable checkpoint",
        content: "The validator recorded a durable judgment for the WP and tied it to the candidate under review.",
      },
      {
        session_id: "WP_VALIDATOR-20260422-100000",
        role: "WP_VALIDATOR",
        checkpoint_type: "SESSION_CLOSE",
        wp_id: "",
        topic: "(auto-closed by new session open)",
        content: "Previous session was not explicitly closed. Auto-closed when new session started.",
        decisions: "(none — auto-closed)",
      },
    ],
  });

  assert.equal(coverage.state, "DEBT");
  assert.deepEqual(coverage.debt_keys, ["WP_VALIDATOR:NO_SESSION_CLOSE"]);
  assert.deepEqual(coverage.role_details[0].explicit_close_session_ids, []);
});

test("repomem coverage flags fragmented proof when open close and durable checkpoints live in different sessions", () => {
  const coverage = evaluateWpRepomemCoverage({
    wpId: "WP-TEST-COVERAGE-v1",
    controlRequests: [
      {
        wp_id: "WP-TEST-COVERAGE-v1",
        role: "INTEGRATION_VALIDATOR",
      },
    ],
    conversationEntries: [
      {
        session_id: "INTEGRATION_VALIDATOR-OPEN",
        role: "INTEGRATION_VALIDATOR",
        checkpoint_type: "SESSION_OPEN",
        wp_id: "",
        topic: "open",
        content: "The final-lane session opened to review closeout truth and candidate readiness for the governed WP.",
      },
      {
        session_id: "INTEGRATION_VALIDATOR-DURABLE",
        role: "INTEGRATION_VALIDATOR",
        checkpoint_type: "CONCERN",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "durable",
        content: "The final-lane reviewer recorded a substantive WP-scoped concern that must stay durable for closeout review.",
      },
      {
        session_id: "INTEGRATION_VALIDATOR-CLOSE",
        role: "INTEGRATION_VALIDATOR",
        checkpoint_type: "SESSION_CLOSE",
        wp_id: "",
        topic: "close",
        content: "The final-lane session closed with decisions recorded, but it did not close the same session that held the durable checkpoint.",
      },
    ],
  });

  assert.equal(coverage.state, "DEBT");
  assert.deepEqual(coverage.debt_keys, ["INTEGRATION_VALIDATOR:FRAGMENTED_SESSION_PROOF"]);
});

test("recent repomem coverage WP collection stays scoped to recent governed activity", () => {
  const wpIds = collectRecentRepomemCoverageWpIds({
    sessions: [
      {
        wp_id: "WP-RECENT-A",
        updated_at: "2026-04-22T09:00:00Z",
      },
      {
        wp_id: "WP-OLD-B",
        updated_at: "2026-03-01T09:00:00Z",
      },
    ],
    controlRequests: [
      {
        wp_id: "WP-RECENT-B",
        created_at: "2026-04-22T08:30:00Z",
      },
    ],
    controlResults: [
      {
        wp_id: "WP-RECENT-C",
        completed_at: "2026-04-22T07:30:00Z",
      },
    ],
    sinceDate: "2026-04-20T00:00:00Z",
  });

  assert.deepEqual(wpIds, ["WP-RECENT-A", "WP-RECENT-B", "WP-RECENT-C"]);
});
