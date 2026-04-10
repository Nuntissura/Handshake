import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  buildWpCommunicationTemplateReplacements,
  findUnreplacedTemplateTokens,
  reconcileWpCommunicationTruth,
} from "../scripts/wp/ensure-wp-communications.mjs";
import { evaluateWpCommunicationHealth } from "../scripts/lib/wp-communication-health-lib.mjs";

const repoRoot = path.resolve(".");

function templateTokens(text) {
  return [...new Set(String(text || "").match(/\{\{[A-Z0-9_]+\}\}/g) || [])].sort();
}

test("WP communication template replacements cover all current template tokens", () => {
  const replacements = buildWpCommunicationTemplateReplacements({
    wpId: "WP-TEST-COMMS-v1",
    baseWpId: "WP-TEST-COMMS",
    dateIso: "2026-03-31T20:00:00.000Z",
    workflowLane: "ORCHESTRATOR_MANAGED",
    executionOwner: "CODER_A",
    workflowAuthority: "ORCHESTRATOR",
    technicalAdvisor: "WP_VALIDATOR",
    technicalAuthority: "INTEGRATION_VALIDATOR",
    mergeAuthority: "INTEGRATION_VALIDATOR",
    wpValidatorOfRecord: "null",
    integrationValidatorOfRecord: "null",
    secondaryValidatorSessionsJson: "[]",
    localBranch: "feat/WP-TEST-COMMS-v1",
    localWorktreeDir: "../wtc-test-comms-v1",
    agenticMode: "NO",
    packetStatus: "Ready for Dev",
    mainContainmentStatusJson: "\"NOT_STARTED\"",
    mergedMainCommitJson: "null",
    mainContainmentVerifiedAtUtcJson: "null",
    currentMainCompatibilityStatusJson: "\"NOT_RUN\"",
    currentMainCompatibilityBaselineShaJson: "null",
    currentMainCompatibilityVerifiedAtUtcJson: "null",
    packetWideningDecisionJson: "\"NONE\"",
    packetWideningEvidenceJson: "null",
    currentTaskBoardStatusJson: "\"READY_FOR_DEV\"",
    currentMilestoneJson: "\"BOOTSTRAP\"",
    taskPacketPath: ".GOV/task_packets/WP-TEST-COMMS-v1/packet.md",
    communicationDir: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1",
    threadFile: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/THREAD.md",
    runtimeStatusFile: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/RUNTIME_STATUS.json",
    receiptsFile: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/RECEIPTS.jsonl",
    heartbeatIntervalMinutes: "15",
    heartbeatDueAt: "2026-03-31T20:15:00.000Z",
    staleAfter: "2026-03-31T20:45:00.000Z",
    maxCoderRevisionCycles: "3",
    maxValidatorReviewCycles: "3",
    maxRelayEscalationCycles: "2",
  });

  const templates = [
    ".GOV/templates/WP_COMMUNICATION_THREAD_TEMPLATE.md",
    ".GOV/templates/WP_RUNTIME_STATUS_TEMPLATE.json",
    ".GOV/templates/WP_RECEIPTS_TEMPLATE.jsonl",
  ].map((relPath) => fs.readFileSync(path.resolve(repoRoot, relPath), "utf8"));

  const uncovered = new Set();
  for (const token of templates.flatMap((text) => templateTokens(text))) {
    if (!(token in replacements)) uncovered.add(token);
  }

  assert.deepEqual([...uncovered], []);
});

test("findUnreplacedTemplateTokens returns sorted unique placeholders", () => {
  assert.deepEqual(
    findUnreplacedTemplateTokens("alpha {{SECOND}} beta {{FIRST}} {{SECOND}}"),
    ["{{FIRST}}", "{{SECOND}}"],
  );
  assert.deepEqual(findUnreplacedTemplateTokens("plain text only"), []);
});

test("reconcileWpCommunicationTruth replays final review receipts into packet and runtime truth", () => {
  const packetText = [
    "# Task Packet: WP-TEST-COMMS-v1",
    "",
    "## METADATA",
    "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
    "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1",
    "- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING",
    "- PACKET_FORMAT_VERSION: 2026-03-29",
    "- EXECUTION_OWNER: CODER_A",
    "- WORKFLOW_AUTHORITY: ORCHESTRATOR",
    "- TECHNICAL_ADVISOR: WP_VALIDATOR",
    "- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR",
    "- MERGE_AUTHORITY: INTEGRATION_VALIDATOR",
    "- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1",
    "- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/THREAD.md",
    "- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/RUNTIME_STATUS.json",
    "- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/RECEIPTS.jsonl",
    "- LOCAL_BRANCH: feat/WP-TEST-COMMS-v1",
    "- LOCAL_WORKTREE_DIR: ../wtc-test-comms-v1",
    "- AGENTIC_MODE: NO",
    "- WP_VALIDATOR_OF_RECORD: wp_validator:test-session",
    "- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:test-session",
    "- SECONDARY_VALIDATOR_SESSIONS: NONE",
    "- **Status:** In Progress",
    "- MAIN_CONTAINMENT_STATUS: NOT_STARTED",
    "- MERGED_MAIN_COMMIT: NONE",
    "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A",
    "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
    "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE",
    "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A",
    "- PACKET_WIDENING_DECISION: NONE",
    "- PACKET_WIDENING_EVIDENCE: N/A",
    "",
    "## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)",
    "Verdict: PENDING",
    "Blockers: Awaiting the final direct review exchange with INTEGRATION_VALIDATOR.",
    "Next: CODER initiates the final direct review exchange with INTEGRATION_VALIDATOR.",
    "",
  ].join("\n");

  const runtimeStatus = {
    schema_version: "wp_runtime_status@1",
    wp_id: "WP-TEST-COMMS-v1",
    current_packet_status: "In Progress",
    runtime_status: "working",
    current_phase: "VALIDATION",
    next_expected_actor: "CODER",
    next_expected_session: "coder:test-session",
    waiting_on: "FINAL_REVIEW_EXCHANGE",
    waiting_on_session: null,
    validator_trigger: "NONE",
    validator_trigger_reason: null,
    ready_for_validation: false,
    ready_for_validation_reason: null,
    attention_required: false,
    open_review_items: [],
    wp_validator_of_record: null,
    integration_validator_of_record: null,
    secondary_validator_sessions: [],
    last_event: "receipt_review_request",
    last_event_at: "2026-04-01T02:17:03.386Z",
    main_containment_status: "NOT_STARTED",
    merged_main_commit: null,
    main_containment_verified_at_utc: null,
    current_main_compatibility_status: "NOT_RUN",
    current_main_compatibility_baseline_sha: null,
    current_main_compatibility_verified_at_utc: null,
    packet_widening_decision: null,
    packet_widening_evidence: null,
  };

  const receipts = [
    {
      receipt_kind: "VALIDATOR_KICKOFF",
      actor_role: "WP_VALIDATOR",
      actor_session: "wp_validator:test-session",
      target_role: "CODER",
      target_session: "coder:test-session",
      correlation_id: "kickoff-1",
      timestamp_utc: "2026-04-01T01:00:00.000Z",
      summary: "kickoff",
    },
    {
      receipt_kind: "CODER_INTENT",
      actor_role: "CODER",
      actor_session: "coder:test-session",
      target_role: "WP_VALIDATOR",
      target_session: "wp_validator:test-session",
      correlation_id: "kickoff-1",
      ack_for: "kickoff-1",
      timestamp_utc: "2026-04-01T01:05:00.000Z",
      summary: "intent",
    },
    {
      receipt_kind: "CODER_HANDOFF",
      actor_role: "CODER",
      actor_session: "coder:test-session",
      target_role: "WP_VALIDATOR",
      target_session: "wp_validator:test-session",
      correlation_id: "handoff-1",
      timestamp_utc: "2026-04-01T01:20:00.000Z",
      summary: "handoff",
    },
    {
      receipt_kind: "VALIDATOR_REVIEW",
      actor_role: "WP_VALIDATOR",
      actor_session: "wp_validator:test-session",
      target_role: "CODER",
      target_session: "coder:test-session",
      correlation_id: "handoff-1",
      ack_for: "handoff-1",
      timestamp_utc: "2026-04-01T01:30:00.000Z",
      summary: "PASS. ready for final review.",
    },
    {
      receipt_kind: "REVIEW_REQUEST",
      actor_role: "CODER",
      actor_session: "coder:test-session",
      target_role: "INTEGRATION_VALIDATOR",
      target_session: "integration_validator:test-session",
      correlation_id: "final-review-1",
      timestamp_utc: "2026-04-01T02:17:03.386Z",
      summary: "final review requested",
    },
    {
      receipt_kind: "REVIEW_RESPONSE",
      actor_role: "INTEGRATION_VALIDATOR",
      actor_session: "integration_validator:test-session",
      target_role: "CODER",
      target_session: "coder:test-session",
      correlation_id: "final-review-1",
      ack_for: "final-review-1",
      timestamp_utc: "2026-04-01T02:46:32.499Z",
      summary: "PASS. No further coder repair is requested.",
    },
  ];

  const reconciliation = reconcileWpCommunicationTruth({
    wpId: "WP-TEST-COMMS-v1",
    packetPath: ".GOV/task_packets/WP-TEST-COMMS-v1/packet.md",
    packetText,
    runtimeStatus,
    receipts,
  });

  assert.match(reconciliation.nextPacketText, /Blockers:\s*NONE/);
  assert.match(
    reconciliation.nextPacketText,
    /Next:\s*ORCHESTRATOR advances verdict progression and integration closeout from the authoritative completed direct-review lane\./,
  );
  assert.equal(reconciliation.nextRuntimeStatus.next_expected_actor, "ORCHESTRATOR");
  assert.equal(reconciliation.nextRuntimeStatus.waiting_on, "VERDICT_PROGRESSION");
  assert.equal(reconciliation.nextRuntimeStatus.current_phase, "VALIDATION");
  assert.equal(reconciliation.nextRuntimeStatus.current_milestone, "VERDICT");
  assert.equal(reconciliation.nextRuntimeStatus.runtime_status, "working");
  assert.equal(reconciliation.nextRuntimeStatus.wp_validator_of_record, "wp_validator:test-session");
  assert.equal(
    reconciliation.nextRuntimeStatus.integration_validator_of_record,
    "integration_validator:test-session",
  );
  assert.equal(reconciliation.nextRuntimeStatus.last_event, "receipt_review_response");
  assert.equal(reconciliation.nextRuntimeStatus.last_event_at, "2026-04-01T02:46:32.499Z");
});

test("reconcileWpCommunicationTruth resets relay cycle after route progress clears a stale validator wake", () => {
  const packetText = [
    "# Task Packet: WP-TEST-COMMS-v1",
    "",
    "## METADATA",
    "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
    "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1",
    "- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING",
    "- PACKET_FORMAT_VERSION: 2026-03-29",
    "- EXECUTION_OWNER: CODER_A",
    "- WORKFLOW_AUTHORITY: ORCHESTRATOR",
    "- TECHNICAL_ADVISOR: WP_VALIDATOR",
    "- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR",
    "- MERGE_AUTHORITY: INTEGRATION_VALIDATOR",
    "- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1",
    "- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/THREAD.md",
    "- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/RUNTIME_STATUS.json",
    "- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/RECEIPTS.jsonl",
    "- LOCAL_BRANCH: feat/WP-TEST-COMMS-v1",
    "- LOCAL_WORKTREE_DIR: ../wtc-test-comms-v1",
    "- AGENTIC_MODE: NO",
    "- **Status:** In Progress",
    "",
    "## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)",
    "Verdict: PENDING",
    "Blockers: Awaiting validator review progress.",
    "Next: WP_VALIDATOR answers the current review request.",
    "",
  ].join("\n");

  const runtimeStatus = {
    schema_version: "wp_runtime_status@1",
    wp_id: "WP-TEST-COMMS-v1",
    current_packet_status: "In Progress",
    runtime_status: "working",
    current_phase: "VALIDATION",
    next_expected_actor: "WP_VALIDATOR",
    next_expected_session: "wp_validator:test-session",
    waiting_on: "OPEN_REVIEW_ITEM_REVIEW_REQUEST",
    waiting_on_session: "wp_validator:test-session",
    validator_trigger: "BLOCKED_NEEDS_VALIDATOR",
    validator_trigger_reason: "REVIEW_REQUEST requires WP_VALIDATOR response",
    attention_required: true,
    ready_for_validation: false,
    ready_for_validation_reason: null,
    open_review_items: [
      {
        correlation_id: "review-1",
        receipt_kind: "REVIEW_REQUEST",
        summary: "review current MT",
        opened_by_role: "CODER",
        opened_by_session: "coder:test-session",
        target_role: "WP_VALIDATOR",
        target_session: "wp_validator:test-session",
        requires_ack: true,
        opened_at: "2026-04-01T02:00:00.000Z",
        updated_at: "2026-04-01T02:00:00.000Z",
      },
    ],
    current_relay_escalation_cycle: 2,
    max_relay_escalation_cycles: 2,
  };

  const receipts = [
    {
      receipt_kind: "VALIDATOR_KICKOFF",
      actor_role: "WP_VALIDATOR",
      actor_session: "wp_validator:test-session",
      target_role: "CODER",
      target_session: "coder:test-session",
      correlation_id: "kickoff-1",
      timestamp_utc: "2026-04-01T01:00:00.000Z",
      summary: "kickoff",
    },
    {
      receipt_kind: "CODER_INTENT",
      actor_role: "CODER",
      actor_session: "coder:test-session",
      target_role: "WP_VALIDATOR",
      target_session: "wp_validator:test-session",
      correlation_id: "kickoff-1",
      ack_for: "kickoff-1",
      timestamp_utc: "2026-04-01T01:05:00.000Z",
      summary: "intent",
    },
    {
      receipt_kind: "REVIEW_REQUEST",
      actor_role: "CODER",
      actor_session: "coder:test-session",
      target_role: "WP_VALIDATOR",
      target_session: "wp_validator:test-session",
      correlation_id: "review-1",
      timestamp_utc: "2026-04-01T02:00:00.000Z",
      summary: "review current MT",
    },
    {
      receipt_kind: "REVIEW_RESPONSE",
      actor_role: "WP_VALIDATOR",
      actor_session: "wp_validator:test-session",
      target_role: "CODER",
      target_session: "coder:test-session",
      correlation_id: "review-1",
      ack_for: "review-1",
      timestamp_utc: "2026-04-01T02:10:00.000Z",
      summary: "PASS. proceed to the next microtask.",
    },
  ];

  const reconciliation = reconcileWpCommunicationTruth({
    wpId: "WP-TEST-COMMS-v1",
    packetPath: ".GOV/task_packets/WP-TEST-COMMS-v1/packet.md",
    packetText,
    runtimeStatus,
    receipts,
  });

  assert.equal(reconciliation.nextRuntimeStatus.current_relay_escalation_cycle, 0);
  assert.equal(reconciliation.nextRuntimeStatus.last_event, "receipt_review_response");
});

test("startup communication health passes when the role-scoped mesh peers are active", () => {
  const evaluation = evaluateWpCommunicationHealth({
    wpId: "WP-TEST-COMMS-v1",
    stage: "STARTUP",
    actorRole: "CODER",
    actorSession: "coder:test-session",
    packetPath: ".GOV/task_packets/WP-TEST-COMMS-v1/packet.md",
    workflowLane: "ORCHESTRATOR_MANAGED",
    packetFormatVersion: "2026-03-29",
    communicationContract: "DIRECT_REVIEW_V1",
    communicationHealthGate: "HANDOFF_VERDICT_BLOCKING",
    receipts: [],
    runtimeStatus: {
      next_expected_actor: "ORCHESTRATOR",
      active_role_sessions: [
        {
          role: "CODER",
          session_id: "coder:test-session",
          state: "working",
          last_heartbeat_at: "2026-04-01T01:00:00.000Z",
        },
        {
          role: "WP_VALIDATOR",
          session_id: "wp_validator:test-session",
          state: "waiting",
          last_heartbeat_at: "2026-04-01T01:00:01.000Z",
        },
      ],
      open_review_items: [],
    },
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.state, "COMM_OK");
  assert.match(evaluation.message, /Startup communication mesh is ready for CODER/);
});

test("startup communication health fails when a required peer session is missing", () => {
  const evaluation = evaluateWpCommunicationHealth({
    wpId: "WP-TEST-COMMS-v1",
    stage: "STARTUP",
    actorRole: "WP_VALIDATOR",
    actorSession: "wp_validator:test-session",
    packetPath: ".GOV/task_packets/WP-TEST-COMMS-v1/packet.md",
    workflowLane: "ORCHESTRATOR_MANAGED",
    packetFormatVersion: "2026-03-29",
    communicationContract: "DIRECT_REVIEW_V1",
    communicationHealthGate: "HANDOFF_VERDICT_BLOCKING",
    receipts: [],
    runtimeStatus: {
      next_expected_actor: "ORCHESTRATOR",
      active_role_sessions: [
        {
          role: "WP_VALIDATOR",
          session_id: "wp_validator:test-session",
          state: "waiting",
          last_heartbeat_at: "2026-04-01T01:00:01.000Z",
        },
      ],
      open_review_items: [],
    },
  });

  assert.equal(evaluation.ok, false);
  assert.equal(evaluation.state, "COMM_MISCONFIGURED");
  assert.match(evaluation.details.join("\n"), /startup_peer_missing=CODER/);
});
