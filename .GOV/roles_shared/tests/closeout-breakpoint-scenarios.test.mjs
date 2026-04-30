import assert from "node:assert/strict";
import test from "node:test";

import { buildCloseoutDependencyView } from "../scripts/lib/wp-closeout-dependency-lib.mjs";
import {
  buildTerminalCloseoutRecordFromCloseoutSync,
  resolveTerminalCloseoutPublication,
} from "../scripts/lib/terminal-closeout-record-lib.mjs";

const WP_ID = "WP-TEST-CLOSEOUT-BREAKPOINTS-v1";

function packet({
  status = "Validated (PASS)",
  verdict = "PASS",
  reportTimestamp = "2026-04-30T11:00:00Z",
} = {}) {
  return [
    `# Task Packet: ${WP_ID}`,
    "",
    `**Status:** ${status}`,
    "",
    "## METADATA",
    `- WP_ID: ${WP_ID}`,
    "",
    "## VALIDATION_REPORTS",
    `### ${reportTimestamp} | INTEGRATION_VALIDATOR | session=intv:breakpoint`,
    `Verdict: ${verdict}`,
  ].join("\n");
}

function runtimeAuthority({
  packetStatus = "Validated (PASS)",
  taskBoardStatus = "DONE_VALIDATED",
  mainContainmentStatus = "CONTAINED_IN_MAIN",
  mergedMainCommit = "0123456789abcdef0123456789abcdef01234567",
} = {}) {
  return {
    current_packet_status: packetStatus,
    current_task_board_status: taskBoardStatus,
    main_containment_status: mainContainmentStatus,
    merged_main_commit: mergedMainCommit,
    execution_state: {
      schema_version: "wp_execution_state@1",
      authority: {
        packet_status: packetStatus,
        task_board_status: taskBoardStatus,
        main_containment_status: mainContainmentStatus,
        merged_main_commit: mergedMainCommit,
        route_anchor: {},
        review_anchor: {},
      },
    },
  };
}

function terminalRecord({
  mode = "CONTAINED_IN_MAIN",
  packetStatus = "Validated (PASS)",
  taskBoardStatus = "DONE_VALIDATED",
  mainContainmentStatus = "CONTAINED_IN_MAIN",
  verdict = "PASS",
  governanceDebtKeys = [],
} = {}) {
  return buildTerminalCloseoutRecordFromCloseoutSync({
    wpId: WP_ID,
    mode,
    packetStatus,
    taskBoardStatus,
    mainContainmentStatus,
    mergedMainCommit: mainContainmentStatus === "CONTAINED_IN_MAIN"
      ? "0123456789abcdef0123456789abcdef01234567"
      : "NONE",
    verdict,
    governanceDebtKeys,
    terminalPublicationRecorded: true,
    actorRole: "INTEGRATION_VALIDATOR",
    actorSession: "intv:breakpoint",
    recordedAtUtc: "2026-04-30T11:05:00Z",
  });
}

function presentTerminalRecord(record) {
  return {
    status: "PRESENT",
    path: "fixture/TERMINAL_CLOSEOUT_RECORD.json",
    record,
    errors: [],
  };
}

function baseView(overrides = {}) {
  return buildCloseoutDependencyView({
    wpId: WP_ID,
    packetContent: packet(overrides.packet || {}),
    runtimeStatus: runtimeAuthority(overrides.runtime || {}),
    taskBoardStatus: overrides.taskBoardStatus ?? "DONE_VALIDATED",
    closeoutRequirements: {
      requireReadyForPass: true,
      requireRecordedScopeCompatibility: true,
      terminalNonPass: false,
      ...(overrides.closeoutRequirements || {}),
    },
    topology: {
      ok: true,
      resolvedWorktreeAbs: "../handshake_main",
      targetHeadSha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      currentMainHeadSha: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      ...(overrides.topology || {}),
    },
    closeoutBundle: {
      ok: true,
      summary: {
        request_count: 1,
        result_count: 1,
        session_count: 1,
        active_run_count: 0,
      },
      ...(overrides.closeoutBundle || {}),
    },
    scopeCompatibility: overrides.scopeCompatibility || {
      parsed: {
        currentMainCompatibilityStatus: "COMPATIBLE",
        currentMainCompatibilityBaselineSha: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        currentMainCompatibilityVerifiedAtUtc: "2026-04-30T11:00:00Z",
      },
      errors: [],
    },
    candidateSignedScope: overrides.candidateSignedScope || { errors: [] },
    terminalCloseoutRecord: overrides.terminalCloseoutRecord,
    repomemCoverage: overrides.repomemCoverage || {
      state: "PASS",
      active_roles: ["ORCHESTRATOR", "INTEGRATION_VALIDATOR"],
      debt_roles: [],
      debt_keys: [],
    },
  });
}

test("breakpoint: PASS with projection debt keeps terminal record authoritative", () => {
  const view = baseView({
    packet: { status: "In Progress", verdict: "PASS" },
    runtime: {
      packetStatus: "Validated (PASS)",
      taskBoardStatus: "DONE_VALIDATED",
    },
    taskBoardStatus: "READY_FOR_DEV",
    terminalCloseoutRecord: presentTerminalRecord(terminalRecord()),
  });

  assert.equal(view.terminal_closeout_record.status, "PRESENT");
  assert.equal(view.settlement.terminal_state, "TERMINAL_SETTLED");
  assert.equal(view.publication.verdict_of_record, "PASS");
  assert.deepEqual(view.product_outcome_blocking_keys, []);
  assert.match(view.governance_debt_keys.join(","), /PACKET_PROJECTION_DRIFT/);
  assert.match(view.governance_debt_keys.join(","), /TASK_BOARD_PROJECTION_DRIFT/);
});

test("breakpoint: stale task-board projection is governance debt only", () => {
  const view = baseView({
    taskBoardStatus: "READY_FOR_DEV",
    terminalCloseoutRecord: presentTerminalRecord(terminalRecord()),
  });

  assert.equal(view.publication.task_board_projection_drift, true);
  assert.deepEqual(view.product_outcome_blocking_keys, []);
  assert.match(view.governance_debt_keys.join(","), /TASK_BOARD_PROJECTION_DRIFT/);
});

test("breakpoint: stale READY residue does not become product outcome failure", () => {
  const view = baseView({
    closeoutBundle: {
      ok: false,
      issues: ["Session WP_VALIDATOR:fixture still reports READY after terminal verdict."],
      summary: { request_count: 1, result_count: 1, session_count: 1, active_run_count: 0 },
    },
    terminalCloseoutRecord: presentTerminalRecord(terminalRecord({ governanceDebtKeys: ["closeout_bundle"] })),
  });

  assert.equal(view.ok, false);
  assert.equal(view.outcome_ok, true);
  assert.deepEqual(view.product_outcome_blocking_keys, []);
  assert.match(view.governance_debt_keys.join(","), /closeout_bundle/);
});

test("breakpoint: stale product-main compatibility proof remains a product blocker", () => {
  const view = baseView({
    scopeCompatibility: {
      parsed: {
        currentMainCompatibilityStatus: "COMPATIBLE",
        currentMainCompatibilityBaselineSha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      },
      errors: ["CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA is stale for current main."],
    },
    terminalCloseoutRecord: presentTerminalRecord(terminalRecord()),
  });

  assert.equal(view.outcome_ok, false);
  assert.deepEqual(view.product_outcome_blocking_keys, ["scope_compatibility"]);
});

test("breakpoint: signed-scope mismatch remains a product blocker", () => {
  const view = baseView({
    candidateSignedScope: {
      errors: ["signed-scope mismatch: committed target touched an undeclared file."],
    },
    terminalCloseoutRecord: presentTerminalRecord(terminalRecord()),
  });

  assert.equal(view.outcome_ok, false);
  assert.deepEqual(view.product_outcome_blocking_keys, ["candidate_target"]);
});

test("breakpoint: missing legacy terminal record falls back to verdict with settlement debt", () => {
  const view = baseView({
    packet: { status: "In Progress", verdict: "FAIL" },
    runtime: {
      packetStatus: "In Progress",
      taskBoardStatus: "IN_PROGRESS",
      mainContainmentStatus: "NOT_STARTED",
      mergedMainCommit: "",
    },
    closeoutRequirements: {
      requireReadyForPass: false,
      requireRecordedScopeCompatibility: false,
      terminalNonPass: true,
    },
    terminalCloseoutRecord: {
      status: "ABSENT",
      path: "fixture/TERMINAL_CLOSEOUT_RECORD.json",
      record: null,
      errors: [],
    },
  });

  assert.equal(view.publication.verdict_of_record, "FAIL");
  assert.equal(view.terminal_closeout_record.status, "MISSING");
  assert.equal(view.settlement.terminal_state, "SETTLEMENT_DEBT");
  assert.match(view.governance_debt_keys.join(","), /TERMINAL_CLOSEOUT_RECORD_MISSING/);
});

test("breakpoint: heavy-host timeout is settlement debt, not revalidation", () => {
  const view = baseView({
    closeoutBundle: {
      ok: false,
      issues: ["heavy-host closeout bundle scan timed out after existing verdict proof."],
      summary: { request_count: 1, result_count: 0, session_count: 1, active_run_count: 0 },
    },
    terminalCloseoutRecord: presentTerminalRecord(terminalRecord({ governanceDebtKeys: ["closeout_bundle"] })),
  });

  assert.equal(view.ok, false);
  assert.equal(view.outcome_ok, true);
  assert.deepEqual(view.product_outcome_blocking_keys, []);
  assert.match(view.governance_debt_keys.join(","), /closeout_bundle/);
});

test("breakpoint: concurrent stale terminal writer is rejected", () => {
  const current = terminalRecord({ mode: "FAIL", packetStatus: "Validated (FAIL)", taskBoardStatus: "DONE_FAIL", mainContainmentStatus: "NOT_REQUIRED", verdict: "FAIL", governanceDebtKeys: ["ACTIVE_TOPOLOGY_ARTIFACT_HYGIENE"] });
  const stale = buildTerminalCloseoutRecordFromCloseoutSync({
    ...current,
    wpId: WP_ID,
    mode: "FAIL",
    packetStatus: "Validated (FAIL)",
    taskBoardStatus: "DONE_FAIL",
    mainContainmentStatus: "NOT_REQUIRED",
    verdict: "FAIL",
    governanceDebtKeys: ["ACTIVE_TOPOLOGY_ARTIFACT_HYGIENE"],
    recordedAtUtc: "2026-04-30T11:04:59Z",
  });

  const decision = resolveTerminalCloseoutPublication({
    currentRecord: current,
    nextRecord: stale,
  });
  assert.equal(decision.ok, false);
  assert.equal(decision.code, "TERMINAL_STALE_WRITER_REJECTED");
});

test("breakpoint: projection-only drift never creates product outcome blockers", () => {
  const view = baseView({
    packet: { status: "Done", verdict: "PASS" },
    runtime: {
      packetStatus: "Validated (PASS)",
      taskBoardStatus: "DONE_VALIDATED",
      mainContainmentStatus: "CONTAINED_IN_MAIN",
    },
    taskBoardStatus: "DONE_MERGE_PENDING",
    terminalCloseoutRecord: presentTerminalRecord(terminalRecord()),
  });

  assert.equal(view.outcome_ok, true);
  assert.deepEqual(view.product_outcome_blocking_keys, []);
  assert.match(view.governance_debt_keys.join(","), /PACKET_PROJECTION_DRIFT|TASK_BOARD_PROJECTION_DRIFT/);
});
