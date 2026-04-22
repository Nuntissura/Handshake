import assert from "node:assert/strict";
import test from "node:test";

import { buildCloseoutDependencyView } from "../scripts/lib/wp-closeout-dependency-lib.mjs";

function packet(status = "Done", {
  verdict = status === "Validated (FAIL)"
    ? "FAIL"
    : status === "Validated (OUTDATED_ONLY)"
      ? "OUTDATED_ONLY"
      : status === "Validated (ABANDONED)"
        ? "ABANDONED"
        : "PASS",
  reportTimestamp = "2026-04-20T10:00:00Z",
  reportSession = "integration-validator-session",
} = {}) {
  return [
    "# Task Packet: WP-TEST-CLOSEOUT-v1",
    "",
    `**Status:** ${status}`,
    "",
    "## METADATA",
    "- WP_ID: WP-TEST-CLOSEOUT-v1",
    "",
    "## VALIDATION_REPORTS",
    `### ${reportTimestamp} | INTEGRATION_VALIDATOR | session=${reportSession}`,
    `Verdict: ${verdict}`,
  ].join("\n");
}

test("closeout dependency view collapses required closeout truth onto explicit dependency statuses", () => {
  const view = buildCloseoutDependencyView({
    packetContent: packet("Done"),
    runtimeStatus: {
      current_packet_status: "Done",
      current_task_board_status: "DONE_MERGE_PENDING",
      main_containment_status: "MERGE_PENDING",
      merged_main_commit: "",
      execution_state: {
        schema_version: "wp_execution_state@1",
        authority: {
          packet_status: "Done",
          task_board_status: "DONE_MERGE_PENDING",
          main_containment_status: "MERGE_PENDING",
        },
      },
    },
    closeoutRequirements: {
      requireReadyForPass: true,
      requireRecordedScopeCompatibility: true,
      terminalNonPass: false,
    },
    topology: {
      ok: true,
      resolvedWorktreeAbs: "../handshake_main",
      targetHeadSha: "abc123",
      currentMainHeadSha: "def456",
    },
    closeoutBundle: {
      ok: true,
      summary: {
        request_count: 1,
        result_count: 1,
        session_count: 1,
        active_run_count: 0,
      },
    },
    scopeCompatibility: {
      parsed: {
        currentMainCompatibilityStatus: "COMPATIBLE",
        currentMainCompatibilityBaselineSha: "0123456789abcdef0123456789abcdef01234567",
        currentMainCompatibilityVerifiedAtUtc: "2026-04-20T10:00:00Z",
      },
      errors: [],
    },
    candidateSignedScope: {
      errors: [],
    },
    closeoutSyncGovernance: {
      latestEvent: {
        mode: "MERGE_PENDING",
        timestamp_utc: "2026-04-20T10:05:00Z",
      },
      latestGovernedAction: {
        rule_id: "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE",
        updated_at: "2026-04-20T10:05:01Z",
      },
    },
  });

  assert.equal(view.ok, true);
  assert.equal(view.publication.closeout_mode, "MERGE_PENDING");
  assert.equal(view.dependencies.topology.status, "PASS");
  assert.equal(view.dependencies.closeout_bundle.status, "PASS");
  assert.equal(view.dependencies.scope_compatibility.status, "PASS");
  assert.equal(view.dependencies.candidate_target.status, "PASS");
  assert.equal(view.dependencies.sync_provenance.status, "RECORDED");
  assert.deepEqual(view.blocking_keys, []);
});

test("closeout dependency view skips signed-scope gating for terminal non-pass packets", () => {
  const view = buildCloseoutDependencyView({
    packetContent: packet("Validated (FAIL)"),
    runtimeStatus: {
      current_packet_status: "Validated (FAIL)",
      current_task_board_status: "DONE_FAIL",
      main_containment_status: "NOT_REQUIRED",
      execution_state: {
        schema_version: "wp_execution_state@1",
        authority: {
          packet_status: "Validated (FAIL)",
          task_board_status: "DONE_FAIL",
          main_containment_status: "NOT_REQUIRED",
        },
      },
    },
    closeoutRequirements: {
      requireReadyForPass: false,
      requireRecordedScopeCompatibility: false,
      terminalNonPass: true,
    },
    topology: {
      ok: true,
      resolvedWorktreeAbs: "../handshake_main",
      targetHeadSha: "abc123",
      currentMainHeadSha: "def456",
    },
    closeoutBundle: {
      ok: true,
      summary: {
        request_count: 0,
        result_count: 0,
        session_count: 0,
        active_run_count: 0,
      },
    },
    scopeCompatibility: {
      errors: ["baseline mismatch should be ignored here"],
    },
    candidateSignedScope: {
      errors: [],
    },
  });

  assert.equal(view.ok, true);
  assert.equal(view.publication.validation_verdict, "FAIL");
  assert.equal(view.publication.closeout_mode, "FAIL");
  assert.equal(view.dependencies.scope_compatibility.status, "SKIP");
  assert.equal(view.settlement.state, "SETTLED");
  assert.deepEqual(view.settlement.blockers, []);
  assert.deepEqual(view.blocking_keys, []);
});

test("closeout dependency view keeps verdict-of-record visible while terminal publication is still debt", () => {
  const view = buildCloseoutDependencyView({
    packetContent: packet("In Progress", {
      verdict: "FAIL",
      reportTimestamp: "2026-04-20T11:15:00Z",
      reportSession: "integration-validator-final",
    }),
    runtimeStatus: {
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      main_containment_status: "NOT_STARTED",
    },
    closeoutRequirements: {
      requireReadyForPass: false,
      requireRecordedScopeCompatibility: false,
      terminalNonPass: true,
    },
    topology: {
      ok: true,
      resolvedWorktreeAbs: "../handshake_main",
      targetHeadSha: "abc123",
      currentMainHeadSha: "def456",
    },
    closeoutBundle: {
      ok: true,
      summary: {
        request_count: 0,
        result_count: 0,
        session_count: 0,
        active_run_count: 0,
      },
    },
    scopeCompatibility: {
      errors: [],
    },
    candidateSignedScope: {
      errors: [],
    },
  });

  assert.equal(view.publication.verdict_of_record, "FAIL");
  assert.equal(view.publication.closeout_mode, "UNSET");
  assert.equal(view.publication.verdict_recorded_at_utc, "2026-04-20T11:15:00Z");
  assert.equal(view.publication.verdict_actor_session, "integration-validator-final");
  assert.equal(view.settlement.state, "SETTLEMENT_DEBT");
  assert.deepEqual(view.settlement.blockers, ["TERMINAL_PUBLICATION_PENDING"]);
});
