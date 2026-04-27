import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repoRoot = path.resolve(".");
const scriptPath = path.join(repoRoot, ".GOV", "roles", "orchestrator", "scripts", "manual-relay-next.mjs");

test("manual-relay-next reports the projected governed next actor for MANUAL_RELAY packets", () => {
  const wpId = "WP-TEST-MANUAL-RELAY-NEXT";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-manual-relay-next-"));
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `# Task Packet: ${wpId}`,
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- WORKFLOW_LANE: MANUAL_RELAY",
      `- WP_RUNTIME_STATUS_FILE: ${runtimePath.replace(/\\/g, "/")}`,
      `- WP_COMMUNICATION_DIR: ${commDir.replace(/\\/g, "/")}`,
    ].join("\n"),
    "utf8",
  );
  fs.writeFileSync(path.join(commDir, "NOTIFICATIONS.jsonl"), "", "utf8");
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      next_expected_actor: "CODER",
      next_expected_session: "coder-test",
      waiting_on: "CODER_HANDOFF",
      runtime_status: "working",
      current_phase: "IMPLEMENTATION",
      current_packet_status: "Ready for Dev",
      current_task_board_status: "READY_FOR_DEV",
    }, null, 2),
    "utf8",
  );

  try {
    const result = spawnSync(process.execPath, [scriptPath, wpId], {
      cwd: repoRoot,
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] workflow_lane=MANUAL_RELAY/);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] lane_owner=CLASSIC_ORCHESTRATOR/);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] next_actor=CODER/);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] next_session=coder-test/);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] dispatch_action=START_SESSION/);
    assert.match(result.stdout, /RELAY_ENVELOPE \[CX-MANUAL-RELAY-001\]/);
    assert.match(result.stdout, /ROLE_TO_ROLE_MESSAGE \[CX-MANUAL-RELAY-002\]/);
    assert.match(result.stdout, /OPERATOR_EXPLAINER \[CX-MANUAL-RELAY-003\]/);
    assert.match(result.stdout, /just manual-relay-dispatch/);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("manual-relay-next classifies role traffic separately from operator explanation", () => {
  const wpId = "WP-TEST-MANUAL-RELAY-ENVELOPE";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-manual-relay-envelope-"));
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");
  const notificationsPath = path.join(commDir, "NOTIFICATIONS.jsonl");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `# Task Packet: ${wpId}`,
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- WORKFLOW_LANE: MANUAL_RELAY",
      `- WP_RUNTIME_STATUS_FILE: ${runtimePath.replace(/\\/g, "/")}`,
      `- WP_COMMUNICATION_DIR: ${commDir.replace(/\\/g, "/")}`,
    ].join("\n"),
    "utf8",
  );
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      next_expected_actor: "WP_VALIDATOR",
      next_expected_session: "wpv-test",
      waiting_on: "WP_VALIDATOR_REVIEW",
      runtime_status: "working",
      current_phase: "VALIDATION",
      current_packet_status: "Ready for Dev",
      current_task_board_status: "READY_FOR_DEV",
      open_review_items: [
        {
          correlation_id: "handoff-1",
          receipt_kind: "CODER_HANDOFF",
          summary: "Ready for validator review.",
          opened_by_role: "CODER",
          opened_by_session: "coder-test",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-test",
          spec_anchor: null,
          packet_row_ref: null,
          requires_ack: true,
          opened_at: "2026-04-05T10:00:00Z",
          updated_at: "2026-04-05T10:00:00Z",
        },
      ],
    }, null, 2),
    "utf8",
  );
  fs.writeFileSync(
    notificationsPath,
    `${JSON.stringify({
      schema_version: "wp_notification@1",
      timestamp_utc: "2026-04-05T10:01:00Z",
      wp_id: wpId,
      source_kind: "CODER_HANDOFF",
      source_role: "CODER",
      source_session: "coder-test",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-test",
      correlation_id: "handoff-1",
      summary: "Ready for validator review.",
    })}\n`,
    "utf8",
  );

  try {
    const result = spawnSync(process.execPath, [scriptPath, wpId], {
      cwd: repoRoot,
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /- FROM: CODER:coder-test/);
    assert.match(result.stdout, /- TO: WP_VALIDATOR:wpv-test/);
    assert.match(result.stdout, /- RELAY_KIND: HANDOFF/);
    assert.match(result.stdout, /- SOURCE_KIND: CODER_HANDOFF/);
    assert.match(result.stdout, /ROLE_TO_ROLE_MESSAGE \[CX-MANUAL-RELAY-002\][\s\S]*Ready for validator review\./);
    assert.match(result.stdout, /OPERATOR_EXPLAINER \[CX-MANUAL-RELAY-003\][\s\S]*Operator is broker-only on MANUAL_RELAY/);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("manual-relay-next accepts classical VALIDATOR and prints validator resume command", () => {
  const wpId = "WP-TEST-MANUAL-RELAY-VALIDATOR";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-manual-relay-validator-"));
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `# Task Packet: ${wpId}`,
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- WORKFLOW_LANE: MANUAL_RELAY",
      `- WP_RUNTIME_STATUS_FILE: ${runtimePath.replace(/\\/g, "/")}`,
      `- WP_COMMUNICATION_DIR: ${commDir.replace(/\\/g, "/")}`,
    ].join("\n"),
    "utf8",
  );
  fs.writeFileSync(path.join(commDir, "NOTIFICATIONS.jsonl"), "", "utf8");
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      next_expected_actor: "VALIDATOR",
      next_expected_session: "validator-test",
      waiting_on: "VALIDATOR_REVIEW",
      runtime_status: "working",
      current_phase: "VALIDATION",
      current_packet_status: "Ready for Dev",
      current_task_board_status: "READY_FOR_DEV",
    }, null, 2),
    "utf8",
  );

  try {
    const result = spawnSync(process.execPath, [scriptPath, wpId], {
      cwd: repoRoot,
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] next_actor=VALIDATOR/);
    assert.match(result.stdout, /next_commands=just validator-next VALIDATOR WP-TEST-MANUAL-RELAY-VALIDATOR/);
    assert.doesNotMatch(result.stdout, /just active-lane-brief VALIDATOR/);
    assert.match(result.stdout, /just manual-relay-dispatch WP-TEST-MANUAL-RELAY-VALIDATOR "relay VALIDATOR/);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("manual-relay-next uses the anchored notification and reports hidden residue counts", () => {
  const wpId = "WP-TEST-MANUAL-RELAY-ANCHOR";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-manual-relay-anchor-"));
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");
  const notificationsPath = path.join(commDir, "NOTIFICATIONS.jsonl");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `# Task Packet: ${wpId}`,
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- WORKFLOW_LANE: MANUAL_RELAY",
      `- WP_RUNTIME_STATUS_FILE: ${runtimePath.replace(/\\/g, "/")}`,
      `- WP_COMMUNICATION_DIR: ${commDir.replace(/\\/g, "/")}`,
    ].join("\n"),
    "utf8",
  );
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      next_expected_actor: "WP_VALIDATOR",
      next_expected_session: "wpv-test",
      waiting_on: "OPEN_REVIEW_ITEM_REVIEW_REQUEST",
      runtime_status: "working",
      current_phase: "VALIDATION",
      current_packet_status: "Ready for Dev",
      current_task_board_status: "READY_FOR_DEV",
      route_anchor_target_role: "WP_VALIDATOR",
      route_anchor_target_session: "wpv-test",
      route_anchor_correlation_id: "handoff-1",
      open_review_items: [
        {
          correlation_id: "handoff-1",
          target_role: "WP_VALIDATOR",
          target_session: "wpv-test",
        },
      ],
    }, null, 2),
    "utf8",
  );
  fs.writeFileSync(
    notificationsPath,
    [
      {
        schema_version: "wp_notification@1",
        timestamp_utc: "2026-04-05T10:02:00Z",
        wp_id: wpId,
        source_kind: "REVIEW_REQUEST",
        source_role: "CODER",
        source_session: "coder-test",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-test",
        correlation_id: "older-handoff",
        summary: "Stale notification should stay hidden.",
      },
      {
        schema_version: "wp_notification@1",
        timestamp_utc: "2026-04-05T10:01:00Z",
        wp_id: wpId,
        source_kind: "REVIEW_REQUEST",
        source_role: "CODER",
        source_session: "coder-test",
        target_role: "WP_VALIDATOR",
        target_session: "wpv-test",
        correlation_id: "handoff-1",
        summary: "Anchored review request should be relayed.",
      },
    ].map((entry) => JSON.stringify(entry)).join("\n") + "\n",
    "utf8",
  );

  try {
    const result = spawnSync(process.execPath, [scriptPath, wpId], {
      cwd: repoRoot,
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] notifications_pending=1/);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] notifications_hidden=1/);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] route_anchor_correlation=handoff-1/);
    assert.match(result.stdout, /ROLE_TO_ROLE_MESSAGE \[CX-MANUAL-RELAY-002\][\s\S]*Anchored review request should be relayed\./);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("manual-relay-next stops when packet and runtime closeout truth drift apart", () => {
  const wpId = "WP-TEST-MANUAL-RELAY-DRIFT";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-manual-relay-drift-"));
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `# Task Packet: ${wpId}`,
      "",
      "## STATUS",
      "- Packet Status: Done",
      "- Task Board: DONE_MERGE_PENDING",
      "- MAIN_CONTAINMENT_STATUS: MERGE_PENDING",
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- WORKFLOW_LANE: MANUAL_RELAY",
      `- WP_RUNTIME_STATUS_FILE: ${runtimePath.replace(/\\/g, "/")}`,
      `- WP_COMMUNICATION_DIR: ${commDir.replace(/\\/g, "/")}`,
    ].join("\n"),
    "utf8",
  );
  fs.writeFileSync(
    runtimePath,
    JSON.stringify({
      next_expected_actor: "CODER",
      next_expected_session: "coder-test",
      waiting_on: "CODER_HANDOFF",
      runtime_status: "working",
      current_phase: "IMPLEMENTATION",
      current_packet_status: "In Progress",
      current_task_board_status: "IN_PROGRESS",
      main_containment_status: "NOT_STARTED",
    }, null, 2),
    "utf8",
  );

  try {
    const result = spawnSync(process.execPath, [scriptPath, wpId], {
      cwd: repoRoot,
      encoding: "utf8",
    });

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Packet\/runtime projection drift blocks manual relay/i);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});
