import assert from "node:assert/strict";
import test from "node:test";
import {
  EXECUTION_STATE_LINEAGE_SCHEMA_VERSION,
  EXECUTION_STATE_SCHEMA_VERSION,
  inferExecutionCloseoutMode,
  inferValidationVerdictFromPublication,
  listExecutionStateCheckpoints,
  materializeRuntimeAuthorityView,
  parseExecutionCloseoutMode,
  readExecutionPublicationView,
  readExecutionProjectionView,
  readRuntimeExecutionAuthority,
  restoreRuntimeExecutionCheckpoint,
  syncRuntimeExecutionState,
} from "../scripts/lib/wp-execution-state-lib.mjs";
import { validateRuntimeStatus } from "../scripts/lib/wp-communications-lib.mjs";

test("syncRuntimeExecutionState captures canonical authority and lineage from runtime fields", () => {
  const runtime = syncRuntimeExecutionState(
    {
      schema_version: "wp_runtime_status@1",
      wp_id: "WP-TEST-EXECUTION-STATE-v1",
      base_wp_id: "WP-TEST-EXECUTION-STATE-v1",
      task_packet: ".GOV/task_packets/WP-TEST-EXECUTION-STATE-v1/packet.md",
      communication_dir: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-EXECUTION-STATE-v1",
      thread_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-EXECUTION-STATE-v1/THREAD.md",
      runtime_status_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-EXECUTION-STATE-v1/RUNTIME_STATUS.json",
      receipts_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-EXECUTION-STATE-v1/RECEIPTS.jsonl",
      workflow_lane: "ORCHESTRATOR_MANAGED",
      execution_owner: "CODER_A",
      workflow_authority: "ORCHESTRATOR",
      technical_advisor: "WP_VALIDATOR",
      technical_authority: "INTEGRATION_VALIDATOR",
      merge_authority: "INTEGRATION_VALIDATOR",
      wp_validator_of_record: "wpv:test",
      integration_validator_of_record: "intv:test",
      secondary_validator_sessions: [],
      agentic_mode: "NO",
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      current_milestone: "MICROTASK",
      runtime_status: "working",
      current_phase: "IMPLEMENTATION",
      next_expected_actor: "CODER",
      next_expected_session: "coder:test",
      waiting_on: "CODER_HANDOFF",
      waiting_on_session: "coder:test",
      validator_trigger: "HANDOFF_READY",
      validator_trigger_reason: "handoff available",
      attention_required: false,
      ready_for_validation: true,
      ready_for_validation_reason: "coder handoff ready",
      current_branch: "feat/WP-TEST-EXECUTION-STATE-v1",
      current_worktree_dir: "../wtc-test-execution-state-v1",
      current_files_touched: [],
      active_role_sessions: [],
      open_review_items: [],
      route_anchor_state: "OPEN_REVIEW",
      route_anchor_kind: "CODER_HANDOFF",
      route_anchor_correlation_id: "handoff-1",
      route_anchor_target_role: "WP_VALIDATOR",
      route_anchor_target_session: "wpv:test",
      authoritative_review_receipt_kind: null,
      authoritative_review_correlation_id: null,
      authoritative_review_actor_session: null,
      authoritative_review_target_session: null,
      authoritative_review_round: null,
      committed_handoff_base_sha: null,
      committed_handoff_head_sha: null,
      committed_handoff_range_source: null,
      last_event: "receipt_coder_handoff",
      last_event_at: "2026-04-19T14:00:00Z",
      last_heartbeat_at: "2026-04-19T14:00:00Z",
      heartbeat_interval_minutes: 15,
      heartbeat_due_at: "2026-04-19T14:15:00Z",
      stale_after: "2026-04-19T14:45:00Z",
      max_coder_revision_cycles: 3,
      max_validator_review_cycles: 3,
      max_relay_escalation_cycles: 3,
      current_coder_revision_cycle: 1,
      current_validator_review_cycle: 0,
      current_relay_escalation_cycle: 0,
      last_backup_push_at: null,
      last_backup_push_sha: null,
    },
    {
      eventName: "receipt_coder_handoff",
      eventAt: "2026-04-19T14:00:00Z",
      checkpointKind: "HANDOFF",
    },
  );

  assert.equal(runtime.execution_state.schema_version, EXECUTION_STATE_SCHEMA_VERSION);
  assert.equal(runtime.execution_state.authority.packet_status, "In Progress");
  assert.equal(runtime.execution_state.authority.task_board_status, "IN_PROGRESS");
  assert.equal(runtime.execution_state.authority.route_anchor.correlation_id, "handoff-1");
  assert.equal(runtime.execution_state.checkpoint_lineage.schema_version, EXECUTION_STATE_LINEAGE_SCHEMA_VERSION);
  assert.equal(runtime.execution_state.checkpoint_lineage.latest_checkpoint_kind, "HANDOFF");
  assert.equal(runtime.execution_state.checkpoint_lineage.checkpoint_count, 1);
  assert.equal(listExecutionStateCheckpoints(runtime).length, 1);
  assert.deepEqual(validateRuntimeStatus(runtime), []);
});

test("restoreRuntimeExecutionCheckpoint rewinds authority and appends a restore checkpoint", () => {
  const initialRuntime = syncRuntimeExecutionState(
    {
      current_packet_status: "Ready for Dev",
      current_task_board_status: "READY_FOR_DEV",
      current_milestone: "STARTUP",
      runtime_status: "submitted",
      current_phase: "BOOTSTRAP",
      next_expected_actor: "WP_VALIDATOR",
      next_expected_session: "wpv:test",
      waiting_on: "VALIDATOR_KICKOFF",
      waiting_on_session: "wpv:test",
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      attention_required: false,
      ready_for_validation: false,
      ready_for_validation_reason: null,
    },
    {
      eventName: "initialize",
      eventAt: "2026-04-19T13:00:00Z",
      checkpointKind: "INITIALIZE",
    },
  );

  const workingRuntime = syncRuntimeExecutionState(
    {
      ...initialRuntime,
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      current_milestone: "MICROTASK",
      runtime_status: "working",
      current_phase: "IMPLEMENTATION",
      next_expected_actor: "CODER",
      next_expected_session: "coder:test",
      waiting_on: "CODER_HANDOFF",
      waiting_on_session: "coder:test",
    },
    {
      eventName: "implementation_started",
      eventAt: "2026-04-19T13:30:00Z",
      checkpointKind: "IMPLEMENTATION",
    },
  );

  const firstCheckpointId = listExecutionStateCheckpoints(workingRuntime)[0].checkpoint_id;
  const restoredRuntime = restoreRuntimeExecutionCheckpoint(workingRuntime, firstCheckpointId, {
    eventName: "restore_to_initialize",
    eventAt: "2026-04-19T14:10:00Z",
  });

  assert.equal(restoredRuntime.current_packet_status, "Ready for Dev");
  assert.equal(restoredRuntime.current_task_board_status, "READY_FOR_DEV");
  assert.equal(restoredRuntime.current_phase, "BOOTSTRAP");
  assert.equal(restoredRuntime.next_expected_actor, "WP_VALIDATOR");
  assert.equal(restoredRuntime.execution_state.checkpoint_lineage.latest_checkpoint_kind, "RESTORE");
  assert.equal(restoredRuntime.execution_state.checkpoint_lineage.checkpoint_count, 3);
});

test("materializeRuntimeAuthorityView prefers canonical execution_state authority over stale flat mirrors", () => {
  const runtime = materializeRuntimeAuthorityView({
    current_packet_status: "Ready for Dev",
    current_task_board_status: "READY_FOR_DEV",
    current_phase: "BOOTSTRAP",
    runtime_status: "submitted",
    next_expected_actor: "WP_VALIDATOR",
    next_expected_session: "wpv:stale",
    waiting_on: "VALIDATOR_KICKOFF",
    waiting_on_session: "wpv:stale",
    execution_state: {
      schema_version: EXECUTION_STATE_SCHEMA_VERSION,
      authority: {
        packet_status: "In Progress",
        task_board_status: "IN_PROGRESS",
        phase: "IMPLEMENTATION",
        runtime_status: "working",
        next_expected_actor: "CODER",
        next_expected_session: "coder:1",
        waiting_on: "CODER_HANDOFF",
        waiting_on_session: "coder:1",
        route_anchor: {
          state: "COMM_WAITING_FOR_REVIEW",
          kind: "CODER_HANDOFF",
          correlation_id: "handoff-2",
          target_role: "WP_VALIDATOR",
          target_session: "wpv:2",
        },
        review_anchor: {},
      },
      checkpoint_lineage: {
        schema_version: EXECUTION_STATE_LINEAGE_SCHEMA_VERSION,
        latest_checkpoint_id: null,
        latest_checkpoint_at_utc: null,
        latest_checkpoint_kind: null,
        latest_restore_point_id: null,
        latest_checkpoint_fingerprint: null,
        checkpoint_count: 0,
        checkpoints: [],
      },
    },
  });

  assert.equal(runtime.current_packet_status, "In Progress");
  assert.equal(runtime.current_task_board_status, "IN_PROGRESS");
  assert.equal(runtime.current_phase, "IMPLEMENTATION");
  assert.equal(runtime.next_expected_actor, "CODER");
  assert.equal(runtime.next_expected_session, "coder:1");
  assert.equal(runtime.route_anchor_correlation_id, "handoff-2");
  assert.equal(runtime.execution_state.authority.next_expected_actor, "CODER");
});

test("materializeRuntimeAuthorityView lets canonical authority clear stale validator route metadata", () => {
  const runtime = materializeRuntimeAuthorityView({
    next_expected_actor: "CODER",
    next_expected_session: "coder:1",
    waiting_on: "CODER_HANDOFF",
    waiting_on_session: "wpv:stale",
    validator_trigger: "BLOCKED_NEEDS_VALIDATOR",
    validator_trigger_reason: "stale validator checkpoint",
    ready_for_validation: true,
    ready_for_validation_reason: "stale validator checkpoint",
    execution_state: {
      schema_version: EXECUTION_STATE_SCHEMA_VERSION,
      authority: {
        next_expected_actor: "CODER",
        next_expected_session: "coder:1",
        waiting_on: "CODER_HANDOFF",
        waiting_on_session: null,
        validator_trigger: "NONE",
        validator_trigger_reason: null,
        ready_for_validation: false,
        ready_for_validation_reason: null,
        route_anchor: {
          state: "COMM_WAITING_FOR_HANDOFF",
          kind: "CODER_HANDOFF",
          correlation_id: "handoff-3",
          target_role: "CODER",
          target_session: "coder:1",
        },
        review_anchor: {},
      },
      checkpoint_lineage: {
        schema_version: EXECUTION_STATE_LINEAGE_SCHEMA_VERSION,
        latest_checkpoint_id: null,
        latest_checkpoint_at_utc: null,
        latest_checkpoint_kind: null,
        latest_restore_point_id: null,
        latest_checkpoint_fingerprint: null,
        checkpoint_count: 0,
        checkpoints: [],
      },
    },
  });

  assert.equal(runtime.waiting_on_session, null);
  assert.equal(runtime.validator_trigger, "NONE");
  assert.equal(runtime.validator_trigger_reason, null);
  assert.equal(runtime.ready_for_validation, false);
  assert.equal(runtime.ready_for_validation_reason, null);
});

test("syncRuntimeExecutionState preserves prior authority when sparse updates omit unrelated route fields", () => {
  const initial = syncRuntimeExecutionState(
    {
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      current_phase: "IMPLEMENTATION",
      runtime_status: "working",
      next_expected_actor: "CODER",
      next_expected_session: "coder:1",
      waiting_on: "CODER_HANDOFF",
      waiting_on_session: "coder:1",
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      ready_for_validation: false,
      ready_for_validation_reason: null,
    },
    {
      eventName: "initial_projection",
      eventAt: "2026-04-21T02:20:00Z",
      checkpointKind: "IMPLEMENTATION",
    },
  );

  const updated = syncRuntimeExecutionState(
    {
      execution_state: initial.execution_state,
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      current_phase: "VALIDATION",
      runtime_status: "working",
    },
    {
      eventName: "sparse_update",
      eventAt: "2026-04-21T02:25:00Z",
      checkpointKind: "REVIEW_SYNC",
    },
  );

  assert.equal(updated.next_expected_actor, "CODER");
  assert.equal(updated.next_expected_session, "coder:1");
  assert.equal(updated.waiting_on, "CODER_HANDOFF");
  assert.equal(updated.waiting_on_session, "coder:1");
});

test("readRuntimeExecutionAuthority falls back to flat runtime fields when canonical block is absent", () => {
  const authority = readRuntimeExecutionAuthority({
    current_packet_status: "Done",
    current_task_board_status: "DONE_MERGE_PENDING",
    current_phase: "STATUS_SYNC",
    runtime_status: "completed",
    next_expected_actor: "INTEGRATION_VALIDATOR",
    next_expected_session: "intv:1",
    waiting_on: "MAIN_CONTAINMENT",
    waiting_on_session: null,
    main_containment_status: "MERGE_PENDING",
  });

  assert.equal(authority.packet_status, "Done");
  assert.equal(authority.task_board_status, "DONE_MERGE_PENDING");
  assert.equal(authority.next_expected_actor, "INTEGRATION_VALIDATOR");
  assert.equal(authority.main_containment_status, "MERGE_PENDING");
});

test("readExecutionProjectionView exposes a narrow canonical status surface", () => {
  const projection = readExecutionProjectionView({
    current_packet_status: "Ready for Dev",
    current_task_board_status: "READY_FOR_DEV",
    runtime_status: "submitted",
    current_phase: "BOOTSTRAP",
    next_expected_actor: "WP_VALIDATOR",
    next_expected_session: "wpv:stale",
    waiting_on: "VALIDATOR_KICKOFF",
    waiting_on_session: "wpv:stale",
    open_review_items: [{ correlation_id: "stale-item" }],
    execution_state: {
      schema_version: EXECUTION_STATE_SCHEMA_VERSION,
      authority: {
        packet_status: "Validated (PASS)",
        task_board_status: "DONE_VALIDATED",
        runtime_status: "completed",
        phase: "STATUS_SYNC",
        milestone: "CONTAINMENT",
        next_expected_actor: "NONE",
        next_expected_session: null,
        waiting_on: "CLOSED",
        waiting_on_session: null,
        route_anchor: {},
        review_anchor: {},
      },
      checkpoint_lineage: {
        schema_version: EXECUTION_STATE_LINEAGE_SCHEMA_VERSION,
        latest_checkpoint_id: null,
        latest_checkpoint_at_utc: null,
        latest_checkpoint_kind: null,
        latest_restore_point_id: null,
        latest_checkpoint_fingerprint: null,
        checkpoint_count: 0,
        checkpoints: [],
      },
    },
  });

  assert.equal(projection.packet_status, "Validated (PASS)");
  assert.equal(projection.task_board_status, "DONE_VALIDATED");
  assert.equal(projection.runtime_status, "completed");
  assert.equal(projection.phase, "STATUS_SYNC");
  assert.equal(projection.open_review_items_count, 1);
  assert.equal(projection.terminal_packet_status, true);
  assert.equal(projection.terminal_task_board_status, true);
  assert.equal(projection.terminal, true);
});

test("readExecutionPublicationView prefers canonical authority for published packet and board status when present", () => {
  const publication = readExecutionPublicationView({
    runtimeStatus: {
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      execution_state: {
        schema_version: EXECUTION_STATE_SCHEMA_VERSION,
        authority: {
          packet_status: "Validated (PASS)",
          task_board_status: "DONE_VALIDATED",
          route_anchor: {},
          review_anchor: {},
        },
        checkpoint_lineage: {
          schema_version: EXECUTION_STATE_LINEAGE_SCHEMA_VERSION,
          latest_checkpoint_id: null,
          latest_checkpoint_at_utc: null,
          latest_checkpoint_kind: null,
          latest_restore_point_id: null,
          latest_checkpoint_fingerprint: null,
          checkpoint_count: 0,
          checkpoints: [],
        },
      },
    },
    packetStatus: "In Progress",
    taskBoardStatus: "READY_FOR_DEV",
  });

  assert.equal(publication.has_canonical_authority, true);
  assert.equal(publication.packet_status, "Validated (PASS)");
  assert.equal(publication.task_board_status, "DONE_VALIDATED");
  assert.equal(publication.packet_projection_drift, true);
  assert.equal(publication.task_board_projection_drift, true);
  assert.equal(publication.terminal, true);
});

test("parseExecutionCloseoutMode normalizes phase and task-board aliases into canonical closeout specs", () => {
  assert.deepEqual(parseExecutionCloseoutMode("done_merge_pending"), {
    mode: "MERGE_PENDING",
    task_board_status: "DONE_MERGE_PENDING",
    packet_status: "Done",
    main_containment_status: "MERGE_PENDING",
    require_merged_main_commit: false,
    required_validation_verdict: "PASS",
  });
  assert.deepEqual(parseExecutionCloseoutMode("CONTAINED_IN_MAIN"), {
    mode: "CONTAINED_IN_MAIN",
    task_board_status: "DONE_VALIDATED",
    packet_status: "Validated (PASS)",
    main_containment_status: "CONTAINED_IN_MAIN",
    require_merged_main_commit: true,
    required_validation_verdict: "PASS",
  });
  assert.equal(parseExecutionCloseoutMode("IN_PROGRESS"), null);
});

test("inferValidationVerdictFromPublication prefers terminal packet and board publication over fallback verdicts", () => {
  assert.equal(
    inferValidationVerdictFromPublication({
      packetStatus: "Done",
      taskBoardStatus: "DONE_VALIDATED",
      fallbackVerdict: "PENDING",
    }),
    "PASS",
  );
  assert.equal(
    inferValidationVerdictFromPublication({
      packetStatus: "In Progress",
      taskBoardStatus: "DONE_FAIL",
      fallbackVerdict: "PENDING",
    }),
    "FAIL",
  );
  assert.equal(
    inferValidationVerdictFromPublication({
      packetStatus: "In Progress",
      taskBoardStatus: "",
      fallbackVerdict: "OUTDATED_ONLY",
    }),
    "OUTDATED_ONLY",
  );
});

test("inferExecutionCloseoutMode derives the correct canonical closeout mode from published truth", () => {
  assert.equal(
    inferExecutionCloseoutMode({
      packetStatus: "Done",
      taskBoardStatus: "DONE_MERGE_PENDING",
      mainContainmentStatus: "MERGE_PENDING",
    })?.mode,
    "MERGE_PENDING",
  );
  assert.equal(
    inferExecutionCloseoutMode({
      packetStatus: "Validated (PASS)",
      taskBoardStatus: "DONE_VALIDATED",
      mainContainmentStatus: "MERGE_PENDING",
    })?.mode,
    "CONTAINED_IN_MAIN",
  );
  assert.equal(
    inferExecutionCloseoutMode({
      packetStatus: "Validated (ABANDONED)",
      taskBoardStatus: "DONE_ABANDONED",
      mainContainmentStatus: "NOT_REQUIRED",
    })?.mode,
    "ABANDONED",
  );
});
