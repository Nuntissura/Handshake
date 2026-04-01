import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  activeWorkflowInvalidityReceipt,
  COMM_ROOT,
  communicationPathsForWp,
  communicationTransactionLockPathForWp,
  validateReceipt,
  validateRuntimeStatus,
} from "../scripts/lib/wp-communications-lib.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const runtimeStatusSchema = JSON.parse(
  fs.readFileSync(path.join(__dirname, "../schemas/WP_RUNTIME_STATUS.schema.json"), "utf8"),
);

function schemaTaskPacketMatches(value) {
  const taskPacketRule = runtimeStatusSchema.properties.task_packet;
  return taskPacketRule.anyOf.some((entry) => new RegExp(entry.pattern).test(value));
}

function runtimeStatusFixture(taskPacket) {
  const wpId = "WP-TEST-RUNTIME-v1";
  const paths = communicationPathsForWp(wpId);
  return {
    schema_version: "wp_runtime_status@1",
    wp_id: wpId,
    base_wp_id: "WP-TEST-RUNTIME",
    task_packet: taskPacket,
    communication_dir: paths.dir,
    thread_file: paths.threadFile,
    runtime_status_file: paths.runtimeStatusFile,
    receipts_file: paths.receiptsFile,
    workflow_lane: "ORCHESTRATOR_MANAGED",
    execution_owner: "CODER_A",
    workflow_authority: "ORCHESTRATOR",
    technical_advisor: "WP_VALIDATOR",
    technical_authority: "INTEGRATION_VALIDATOR",
    merge_authority: "INTEGRATION_VALIDATOR",
    wp_validator_of_record: "wpv-1",
    integration_validator_of_record: "intval-1",
    secondary_validator_sessions: [],
    agentic_mode: "YES",
    current_packet_status: "In Progress",
    main_containment_status: "NOT_STARTED",
    merged_main_commit: null,
    main_containment_verified_at_utc: null,
    runtime_status: "working",
    current_phase: "IMPLEMENTATION",
    next_expected_actor: "WP_VALIDATOR",
    next_expected_session: "wpv-1",
    waiting_on: "WP_VALIDATOR_REVIEW",
    waiting_on_session: "wpv-1",
    validator_trigger: "HANDOFF_READY",
    validator_trigger_reason: "Coder handoff recorded; WP validator review required",
    attention_required: false,
    ready_for_validation: true,
    ready_for_validation_reason: "Coder handoff recorded; WP validator review required",
    current_branch: "feat/WP-TEST-RUNTIME-v1",
    current_worktree_dir: "../wtc-test-runtime-v1",
    current_files_touched: [".GOV/roles_shared/scripts/lib/wp-communications-lib.mjs"],
    active_role_sessions: [
      {
        role: "CODER",
        session_id: "coder-1",
        authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        worktree_dir: "../wtc-test-runtime-v1",
        state: "working",
        last_heartbeat_at: "2026-03-24T10:00:00Z",
      },
    ],
    open_review_items: [],
    last_event: "receipt_coder_handoff",
    last_event_at: "2026-03-24T10:00:00Z",
    last_heartbeat_at: "2026-03-24T10:00:00Z",
    heartbeat_interval_minutes: 15,
    heartbeat_due_at: "2026-03-24T10:15:00Z",
    stale_after: "2026-03-24T10:45:00Z",
    max_coder_revision_cycles: 3,
    max_validator_review_cycles: 3,
    max_relay_escalation_cycles: 2,
    current_coder_revision_cycle: 1,
    current_validator_review_cycle: 0,
    current_relay_escalation_cycle: 0,
    last_backup_push_at: null,
    last_backup_push_sha: null,
  };
}

test("WP runtime schema accepts both flat and folder task packet paths", () => {
  assert.equal(schemaTaskPacketMatches(".GOV/task_packets/WP-TEST-RUNTIME-v1.md"), true);
  assert.equal(schemaTaskPacketMatches(".GOV/task_packets/WP-TEST-RUNTIME-v1/packet.md"), true);
  assert.equal(schemaTaskPacketMatches(".GOV/task_packets/README.md"), false);
});

test("validateRuntimeStatus accepts folder packet paths used by live v3 packets", () => {
  const errors = validateRuntimeStatus(runtimeStatusFixture(".GOV/task_packets/WP-TEST-RUNTIME-v1/packet.md"));
  assert.deepEqual(errors, []);
});

test("communicationTransactionLockPathForWp stays in the shared communication root", () => {
  assert.equal(
    communicationTransactionLockPathForWp("WP-TEST-RUNTIME-v1"),
    `${COMM_ROOT}/WP-TEST-RUNTIME-v1.tx.lock`,
  );
});

function reviewResolutionReceiptFixture(overrides = {}) {
  return {
    schema_version: "wp_receipt@1",
    timestamp_utc: "2026-03-24T10:00:00Z",
    wp_id: "WP-TEST-RUNTIME-v1",
    actor_role: "WP_VALIDATOR",
    actor_session: "wpv-1",
    actor_authority_kind: "WP_VALIDATOR",
    validator_role_kind: "WP_VALIDATOR",
    receipt_kind: "VALIDATOR_REVIEW",
    summary: "Validator review complete",
    branch: "feat/WP-TEST-RUNTIME-v1",
    worktree_dir: "../wtc-test-runtime-v1",
    state_before: null,
    state_after: null,
    target_role: "CODER",
    target_session: "coder-1",
    correlation_id: "handoff-1",
    requires_ack: false,
    ack_for: "handoff-1",
    spec_anchor: null,
    packet_row_ref: null,
    refs: [".GOV/task_packets/WP-TEST-RUNTIME-v1/packet.md"],
    ...overrides,
  };
}

function workflowInvalidityReceiptFixture(overrides = {}) {
  return {
    schema_version: "wp_receipt@1",
    timestamp_utc: "2026-03-24T10:00:00Z",
    wp_id: "WP-TEST-RUNTIME-v1",
    actor_role: "ORCHESTRATOR",
    actor_session: "orch-1",
    actor_authority_kind: "WORKFLOW_AUTHORITY",
    validator_role_kind: null,
    receipt_kind: "WORKFLOW_INVALIDITY",
    summary: "Manual checkpoint helper was invoked for an orchestrator-managed WP",
    branch: "gov_kernel",
    worktree_dir: "../wt-gov-kernel",
    state_before: null,
    state_after: "WORKFLOW_INVALID",
    target_role: "ORCHESTRATOR",
    target_session: null,
    correlation_id: null,
    requires_ack: false,
    ack_for: null,
    spec_anchor: "CX-GATE-001",
    packet_row_ref: null,
    workflow_invalidity_code: "ORCHESTRATOR_MANAGED_CHECKPOINT_RELAPSE",
    refs: [".GOV/task_packets/WP-TEST-RUNTIME-v1/packet.md"],
    ...overrides,
  };
}

function repairReceiptFixture(overrides = {}) {
  return {
    schema_version: "wp_receipt@1",
    timestamp_utc: "2026-03-24T10:05:00Z",
    wp_id: "WP-TEST-RUNTIME-v1",
    actor_role: "ORCHESTRATOR",
    actor_session: "orch-1",
    actor_authority_kind: "WORKFLOW_AUTHORITY",
    validator_role_kind: null,
    receipt_kind: "REPAIR",
    summary: "Scope truth repaired; resume governed lane",
    branch: "gov_kernel",
    worktree_dir: "../wt-gov-kernel",
    state_before: "WORKFLOW_INVALID",
    state_after: "SCOPE_REPAIRED",
    target_role: "CODER",
    target_session: "coder-1",
    correlation_id: null,
    requires_ack: false,
    ack_for: null,
    spec_anchor: "CLAUSE_CLOSURE_MATRIX",
    packet_row_ref: "IN_SCOPE_PATHS",
    refs: [".GOV/task_packets/WP-TEST-RUNTIME-v1/packet.md"],
    ...overrides,
  };
}

test("validateReceipt requires target_session for direct-review receipts", () => {
  const errors = validateReceipt(reviewResolutionReceiptFixture({
    target_session: null,
  }));
  assert.match(errors.join("\n"), /target_session is required for VALIDATOR_REVIEW/);
});

test("validateReceipt requires ack_for to match correlation_id for resolution receipts", () => {
  const errors = validateReceipt(reviewResolutionReceiptFixture({
    ack_for: "wrong-correlation",
  }));
  assert.match(errors.join("\n"), /ack_for must match correlation_id for VALIDATOR_REVIEW/);
});

test("validateReceipt accepts structured microtask contracts on review receipts", () => {
  const errors = validateReceipt(reviewResolutionReceiptFixture({
    microtask_contract: {
      scope_ref: "CLAUSE_CLOSURE_MATRIX/LM-SEARCH-001",
      file_targets: ["src/backend/handshake_core/src/storage/sqlite.rs"],
      proof_commands: ["cargo test storage::tests::sqlite_loom_storage_conformance -- --exact"],
      risk_focus: "portable search parity",
      review_mode: "OVERLAP",
      phase_gate: "MICROTASK",
      review_outcome: "REPAIR_REQUIRED",
      expected_receipt_kind: "REVIEW_RESPONSE",
    },
  }));
  assert.deepEqual(errors, []);
});

test("validateReceipt rejects empty microtask contracts", () => {
  const errors = validateReceipt(reviewResolutionReceiptFixture({
    microtask_contract: {},
  }));
  assert.match(errors.join("\n"), /microtask_contract must contain at least one populated field/);
});

test("receipt schema exposes microtask_contract for external consumers", () => {
  const rule = runtimeStatusSchema.properties.open_review_items.items.properties.microtask_contract;
  assert.equal(rule.type.includes("object"), true);
  assert.equal(rule.properties.expected_receipt_kind.enum.includes("CODER_INTENT"), true);
  assert.equal(rule.properties.review_mode.enum.includes("OVERLAP"), true);
  assert.equal(rule.properties.phase_gate.enum.includes("SKELETON"), true);
  assert.equal(rule.properties.review_outcome.enum.includes("REPAIR_REQUIRED"), true);
});

test("runtime receipt schema exposes microtask_contract on receipts", () => {
  const receiptSchema = JSON.parse(
    fs.readFileSync(path.join(__dirname, "../schemas/WP_RECEIPT.schema.json"), "utf8"),
  );
  const rule = receiptSchema.properties.microtask_contract;
  assert.equal(rule.type.includes("object"), true);
  assert.equal(rule.properties.expected_receipt_kind.enum.includes("WORKFLOW_INVALIDITY"), true);
  assert.equal(rule.properties.review_mode.enum.includes("BLOCKING"), true);
  assert.equal(rule.properties.phase_gate.enum.includes("BOOTSTRAP"), true);
  assert.equal(rule.properties.review_outcome.enum.includes("APPROVED_FOR_FINAL_REVIEW"), true);
});

test("validateReceipt accepts WORKFLOW_INVALIDITY receipts with a machine code", () => {
  const errors = validateReceipt(workflowInvalidityReceiptFixture());
  assert.deepEqual(errors, []);
});

test("validateReceipt requires workflow_invalidity_code for WORKFLOW_INVALIDITY receipts", () => {
  const errors = validateReceipt(workflowInvalidityReceiptFixture({
    workflow_invalidity_code: null,
  }));
  assert.match(errors.join("\n"), /workflow_invalidity_code is required for WORKFLOW_INVALIDITY/);
});

test("activeWorkflowInvalidityReceipt returns the unresolved invalidity when no repair follows", () => {
  const active = activeWorkflowInvalidityReceipt([
    workflowInvalidityReceiptFixture(),
  ]);

  assert.equal(active?.workflow_invalidity_code, "ORCHESTRATOR_MANAGED_CHECKPOINT_RELAPSE");
});

test("activeWorkflowInvalidityReceipt clears earlier invalidity after a later repair receipt", () => {
  const active = activeWorkflowInvalidityReceipt([
    workflowInvalidityReceiptFixture(),
    repairReceiptFixture(),
  ]);

  assert.equal(active, null);
});
