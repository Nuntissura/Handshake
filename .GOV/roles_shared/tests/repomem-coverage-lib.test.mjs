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

test("repomem coverage tracks classic orchestrator and activation manager WP activity", () => {
  const coverage = evaluateWpRepomemCoverage({
    wpId: "WP-TEST-COVERAGE-v1",
    receipts: [
      {
        wp_id: "WP-TEST-COVERAGE-v1",
        actor_role: "CLASSIC_ORCHESTRATOR",
      },
    ],
    sessions: [
      {
        wp_id: "WP-TEST-COVERAGE-v1",
        role: "ACTIVATION_MANAGER",
      },
    ],
    conversationEntries: [
      {
        session_id: "CLASSIC_ORCHESTRATOR-20260422-083000",
        role: "CLASSIC_ORCHESTRATOR",
        checkpoint_type: "SESSION_OPEN",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "classic orchestrator session open",
        content: "The manual relay orchestration session opened with a clear WP-bound scope and operator relay context.",
      },
      {
        session_id: "CLASSIC_ORCHESTRATOR-20260422-083000",
        role: "CLASSIC_ORCHESTRATOR",
        checkpoint_type: "DECISION",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "classic orchestrator durable decision",
        content: "The manual lane recorded a durable relay decision before creating the governed handoff for this WP.",
      },
      {
        session_id: "CLASSIC_ORCHESTRATOR-20260422-083000",
        role: "CLASSIC_ORCHESTRATOR",
        checkpoint_type: "SESSION_CLOSE",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "classic orchestrator session close",
        content: "The manual relay orchestration session closed after capturing the key route decision and outcome.",
      },
      {
        session_id: "ACTIVATION_MANAGER-20260422-084500",
        role: "ACTIVATION_MANAGER",
        checkpoint_type: "SESSION_OPEN",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "activation manager session open",
        content: "The activation manager opened the WP-bound setup session with refinement and readiness context.",
      },
      {
        session_id: "ACTIVATION_MANAGER-20260422-084500",
        role: "ACTIVATION_MANAGER",
        checkpoint_type: "INSIGHT",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "activation manager durable insight",
        content: "The activation manager found a readiness constraint that needed to remain visible to closeout diagnostics.",
      },
      {
        session_id: "ACTIVATION_MANAGER-20260422-084500",
        role: "ACTIVATION_MANAGER",
        checkpoint_type: "SESSION_CLOSE",
        wp_id: "WP-TEST-COVERAGE-v1",
        topic: "activation manager session close",
        content: "The activation manager closed after recording readiness decisions and the durable WP-bound insight.",
      },
    ],
  });

  assert.equal(coverage.state, "PASS");
  assert.deepEqual(coverage.active_roles, ["CLASSIC_ORCHESTRATOR", "ACTIVATION_MANAGER"]);
  assert.deepEqual(coverage.debt_roles, []);
});

test("repomem coverage ignores Memory Manager synthetic hygiene activity for normal WP debt", () => {
  const coverage = evaluateWpRepomemCoverage({
    wpId: "WP-MEMORY-HYGIENE_20260425T010000Z",
    sessions: [
      {
        wp_id: "WP-MEMORY-HYGIENE_20260425T010000Z",
        role: "MEMORY_MANAGER",
      },
    ],
    controlRequests: [
      {
        wp_id: "WP-MEMORY-HYGIENE_20260425T010000Z",
        role: "MEMORY_MANAGER",
      },
    ],
    conversationEntries: [],
  });

  assert.equal(coverage.state, "NO_ACTIVE_ROLES");
  assert.deepEqual(coverage.active_roles, []);
  assert.deepEqual(coverage.debt_roles, []);
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
