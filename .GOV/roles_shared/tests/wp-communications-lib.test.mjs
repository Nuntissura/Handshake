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
  ensurePacketlessWpCommunicationScaffold,
  validateReceipt,
  validateRuntimeStatus,
} from "../scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs } from "../scripts/lib/runtime-paths.mjs";

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
    current_main_compatibility_status: "NOT_RUN",
    current_main_compatibility_baseline_sha: null,
    current_main_compatibility_verified_at_utc: null,
    packet_widening_decision: null,
    packet_widening_evidence: null,
    authoritative_review_receipt_kind: "VALIDATOR_REVIEW",
    authoritative_review_correlation_id: "handoff-1",
    authoritative_review_actor_session: "wpv-1",
    authoritative_review_target_session: "coder-1",
    authoritative_review_round: 1,
    committed_handoff_base_sha: "0123456789abcdef0123456789abcdef01234567",
    committed_handoff_head_sha: "89abcdef0123456789abcdef0123456789abcdef",
    committed_handoff_range_source: "PACKET_EXPLICIT_HANDOFF_RANGE",
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
    route_anchor_state: "COMM_WAITING_FOR_REVIEW",
    route_anchor_kind: "CODER_HANDOFF",
    route_anchor_correlation_id: "handoff-1",
    route_anchor_target_role: "WP_VALIDATOR",
    route_anchor_target_session: "wpv-1",
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
    max_worker_interrupt_cycles: 1,
    current_coder_revision_cycle: 1,
    current_validator_review_cycle: 0,
    current_relay_escalation_cycle: 0,
    current_worker_interrupt_cycle: 0,
    last_backup_push_at: null,
    last_backup_push_sha: null,
  };
}

test("WP runtime schema accepts both task_packets and work_packets packet paths", () => {
  assert.equal(schemaTaskPacketMatches(".GOV/task_packets/WP-TEST-RUNTIME-v1.md"), true);
  assert.equal(schemaTaskPacketMatches(".GOV/task_packets/WP-TEST-RUNTIME-v1/packet.md"), true);
  assert.equal(schemaTaskPacketMatches(".GOV/work_packets/WP-TEST-RUNTIME-v1.md"), true);
  assert.equal(schemaTaskPacketMatches(".GOV/work_packets/WP-TEST-RUNTIME-v1/packet.md"), true);
  assert.equal(schemaTaskPacketMatches(".GOV/task_packets/README.md"), false);
});

test("validateRuntimeStatus accepts folder packet paths used by live v3 packets", () => {
  const errors = validateRuntimeStatus(runtimeStatusFixture(".GOV/task_packets/WP-TEST-RUNTIME-v1/packet.md"));
  assert.deepEqual(errors, []);
});

test("validateRuntimeStatus accepts cross-repo kernel packet paths when they resolve to the authoritative packet", () => {
  const errors = validateRuntimeStatus({
    ...runtimeStatusFixture("../wt-gov-kernel/.GOV/task_packets/WP-1-Session-Observability-Spans-FR-v1/packet.md"),
    wp_id: "WP-1-Session-Observability-Spans-FR-v1",
    base_wp_id: "WP-1-Session-Observability-Spans-FR",
    communication_dir: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Session-Observability-Spans-FR-v1",
    thread_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Session-Observability-Spans-FR-v1/THREAD.md",
    runtime_status_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Session-Observability-Spans-FR-v1/RUNTIME_STATUS.json",
    receipts_file: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Session-Observability-Spans-FR-v1/RECEIPTS.jsonl",
  });
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

function memoryManagerReceiptFixture(overrides = {}) {
  return {
    schema_version: "wp_receipt@1",
    timestamp_utc: "2026-04-09T21:15:00Z",
    wp_id: "WP-MEMORY-HYGIENE_2026-04-09T2115Z",
    actor_role: "MEMORY_MANAGER",
    actor_session: "MEMORY_MANAGER:WP-MEMORY-HYGIENE_2026-04-09T2115Z",
    actor_authority_kind: "MEMORY_MANAGER",
    validator_role_kind: null,
    receipt_kind: "MEMORY_PROPOSAL",
    summary: "Cross-WP failure pattern should become an explicit governance hard gate.",
    branch: "gov_kernel",
    worktree_dir: ".",
    state_before: null,
    state_after: "PROPOSAL_WRITTEN",
    target_role: "ORCHESTRATOR",
    target_session: null,
    correlation_id: "mm-proposal-1",
    requires_ack: false,
    ack_for: null,
    spec_anchor: null,
    packet_row_ref: null,
    refs: [
      "../gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md",
      ".GOV/roles/memory_manager/proposals/test-proposal.md",
    ],
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
      commit: "0123456789abcdef0123456789abcdef01234567",
      file_targets: ["src/backend/handshake_core/src/storage/sqlite.rs"],
      proof_commands: ["cargo test storage::tests::sqlite_loom_storage_conformance -- --exact"],
      risk_focus: "portable search parity",
      review_mode: "OVERLAP",
      phase_gate: "MICROTASK",
      review_outcome: "REPAIR_REQUIRED",
      expected_receipt_kind: "REVIEW_RESPONSE",
      heuristic_risk: "YES",
      heuristic_risk_class: "FUZZY_DISCRIMINATOR",
      required_evidence: ["CORPUS_CASES", "NEGATIVE_COUNTEREXAMPLES"],
      strategy_escalation: "DISCRIMINATOR_REDESIGN",
      repair_cycle_strategy_threshold: 2,
    },
  }));
  assert.deepEqual(errors, []);
});

test("validateReceipt rejects malformed microtask handoff commit fields", () => {
  const errors = validateReceipt(reviewResolutionReceiptFixture({
    microtask_contract: {
      scope_ref: "MT-001",
      commit: "not-a-sha",
    },
  }));
  assert.match(errors.join("\n"), /microtask_contract\.commit must be null or a 40-character SHA/);
});

test("validateReceipt accepts Memory Manager proposal receipts", () => {
  const errors = validateReceipt(memoryManagerReceiptFixture());
  assert.deepEqual(errors, []);
});

test("ensurePacketlessWpCommunicationScaffold creates synthetic communication files without a packet", () => {
  const wpId = "WP-MEMORY-HYGIENE_TEST-SCAFFOLD";
  const paths = communicationPathsForWp(wpId);
  fs.rmSync(repoPathAbs(paths.dir), { recursive: true, force: true });

  try {
    const scaffold = ensurePacketlessWpCommunicationScaffold(wpId, {
      noteLines: ["Synthetic Memory Manager lane."],
    });

    assert.equal(fs.existsSync(repoPathAbs(scaffold.threadFile)), true);
    assert.equal(fs.existsSync(repoPathAbs(scaffold.receiptsFile)), true);
    assert.equal(fs.existsSync(repoPathAbs(scaffold.notificationsFile)), true);
    assert.equal(fs.existsSync(repoPathAbs(scaffold.cursorFile)), true);
  } finally {
    fs.rmSync(repoPathAbs(paths.dir), { recursive: true, force: true });
  }
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
  assert.equal(rule.properties.commit.pattern, "^[0-9a-fA-F]{40}$");
  assert.equal(rule.properties.review_mode.enum.includes("OVERLAP"), true);
  assert.equal(rule.properties.phase_gate.enum.includes("SKELETON"), true);
  assert.equal(rule.properties.review_outcome.enum.includes("REPAIR_REQUIRED"), true);
  assert.equal(rule.properties.heuristic_risk.enum.includes("YES"), true);
  assert.equal(rule.properties.heuristic_risk_class.enum.includes("FUZZY_DISCRIMINATOR"), true);
  assert.equal(rule.properties.strategy_escalation.enum.includes("DISCRIMINATOR_REDESIGN"), true);
});

test("runtime receipt schema exposes microtask_contract on receipts", () => {
  const receiptSchema = JSON.parse(
    fs.readFileSync(path.join(__dirname, "../schemas/WP_RECEIPT.schema.json"), "utf8"),
  );
  const rule = receiptSchema.properties.microtask_contract;
  assert.equal(rule.type.includes("object"), true);
  assert.equal(rule.properties.expected_receipt_kind.enum.includes("WORKFLOW_INVALIDITY"), true);
  assert.equal(rule.properties.commit.pattern, "^[0-9a-fA-F]{40}$");
  assert.equal(rule.properties.review_mode.enum.includes("BLOCKING"), true);
  assert.equal(rule.properties.phase_gate.enum.includes("BOOTSTRAP"), true);
  assert.equal(rule.properties.review_outcome.enum.includes("APPROVED_FOR_FINAL_REVIEW"), true);
  assert.equal(rule.properties.heuristic_risk.enum.includes("YES"), true);
  assert.equal(rule.properties.heuristic_risk_class.enum.includes("SECRET_OR_IDENTIFIER_BOUNDARY"), true);
  assert.equal(rule.properties.strategy_escalation.enum.includes("PROPERTY_BASED_REFRAME"), true);
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
