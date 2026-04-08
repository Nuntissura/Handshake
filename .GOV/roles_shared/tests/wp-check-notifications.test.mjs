import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { checkNotifications } from "../scripts/wp/wp-check-notifications.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

function writePacket(packetDir, commDir) {
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    `- WP_COMMUNICATION_DIR: ${commDir.replace(/\\/g, "/")}\n`,
    "utf8",
  );
}

test("acknowledging one validator session does not clear another session's notifications", () => {
  const wpId = "WP-TEST-NOTIF-SCOPING";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "wp-notif-scope-"));
  const notificationsPath = path.join(commDir, "NOTIFICATIONS.jsonl");
  const cursorPath = path.join(commDir, "NOTIFICATION_CURSOR.json");

  writePacket(packetDir, commDir);
  fs.writeFileSync(cursorPath, `${JSON.stringify({ schema_version: "wp_notification_cursor@1", cursors: {} }, null, 2)}\n`, "utf8");
  fs.writeFileSync(notificationsPath, [
    JSON.stringify({
      schema_version: "wp_notification@1",
      timestamp_utc: "2026-03-24T10:00:00Z",
      wp_id: wpId,
      source_kind: "CODER_HANDOFF",
      source_role: "CODER",
      source_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-a",
      correlation_id: "handoff-a",
      summary: "handoff for validator session a",
    }),
    JSON.stringify({
      schema_version: "wp_notification@1",
      timestamp_utc: "2026-03-24T10:01:00Z",
      wp_id: wpId,
      source_kind: "CODER_HANDOFF",
      source_role: "CODER",
      source_session: "coder-1",
      target_role: "WP_VALIDATOR",
      target_session: "wpv-b",
      correlation_id: "handoff-b",
      summary: "handoff for validator session b",
    }),
  ].join("\n") + "\n", "utf8");

  try {
    const before = checkNotifications({ wpId, role: "WP_VALIDATOR" });
    assert.equal(before.pendingCount, 2);

    const ack = checkNotifications({ wpId, role: "WP_VALIDATOR", ack: true, session: "wpv-a" });
    assert.equal(ack.acknowledged, true);

    const sessionA = checkNotifications({ wpId, role: "WP_VALIDATOR", session: "wpv-a" });
    const sessionB = checkNotifications({ wpId, role: "WP_VALIDATOR", session: "wpv-b" });
    const aggregate = checkNotifications({ wpId, role: "WP_VALIDATOR" });

    assert.equal(sessionA.pendingCount, 0);
    assert.equal(sessionB.pendingCount, 1);
    assert.equal(aggregate.pendingCount, 1);
    assert.deepEqual(sessionB.notifications.map((entry) => entry.target_session), ["wpv-b"]);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("session-scoped checks can consume placeholder <unassigned> notifications once the governed session exists", () => {
  const wpId = "WP-TEST-NOTIF-UNASSIGNED";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "wp-notif-unassigned-"));
  const notificationsPath = path.join(commDir, "NOTIFICATIONS.jsonl");
  const cursorPath = path.join(commDir, "NOTIFICATION_CURSOR.json");

  writePacket(packetDir, commDir);
  fs.writeFileSync(cursorPath, `${JSON.stringify({ schema_version: "wp_notification_cursor@1", cursors: {} }, null, 2)}\n`, "utf8");
  fs.writeFileSync(notificationsPath, `${JSON.stringify({
    schema_version: "wp_notification@1",
    timestamp_utc: "2026-04-01T10:00:00Z",
    wp_id: wpId,
    source_kind: "REVIEW_REQUEST",
    source_role: "CODER",
    source_session: "coder-1",
    target_role: "INTEGRATION_VALIDATOR",
    target_session: "<unassigned>",
    correlation_id: "review-request-a",
    summary: "final review request before integration-validator session was claimed",
  })}\n`, "utf8");

  try {
    const visibleToSession = checkNotifications({
      wpId,
      role: "INTEGRATION_VALIDATOR",
      session: "integration_validator:test",
    });
    assert.equal(visibleToSession.pendingCount, 1);

    const ack = checkNotifications({
      wpId,
      role: "INTEGRATION_VALIDATOR",
      ack: true,
      session: "integration_validator:test",
    });
    assert.equal(ack.acknowledged, true);

    const after = checkNotifications({
      wpId,
      role: "INTEGRATION_VALIDATOR",
      session: "integration_validator:test",
    });
    assert.equal(after.pendingCount, 0);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("active notification view hides terminal residue while history view preserves it", () => {
  const wpId = "WP-TEST-NOTIF-TERMINAL-HISTORY";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = fs.mkdtempSync(path.join(os.tmpdir(), "wp-notif-terminal-history-"));
  const notificationsPath = path.join(commDir, "NOTIFICATIONS.jsonl");
  const cursorPath = path.join(commDir, "NOTIFICATION_CURSOR.json");
  const runtimePath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");

  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `- WP_COMMUNICATION_DIR: ${commDir.replace(/\\/g, "/")}`,
      `- WP_RUNTIME_STATUS_FILE: ${runtimePath.replace(/\\/g, "/")}`,
      `- WP_RECEIPTS_FILE: ${receiptsPath.replace(/\\/g, "/")}`,
      "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
      "- PACKET_FORMAT_VERSION: 2026-03-29",
      "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1",
      "- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING",
      "- **Status:** Validated (PASS)",
    ].join("\n"),
    "utf8",
  );
  fs.writeFileSync(cursorPath, `${JSON.stringify({ schema_version: "wp_notification_cursor@1", cursors: {} }, null, 2)}\n`, "utf8");
  fs.writeFileSync(
    runtimePath,
    `${JSON.stringify({
      current_packet_status: "Validated (PASS)",
      runtime_status: "completed",
      next_expected_actor: "NONE",
      next_expected_session: null,
      waiting_on: "CLOSED",
      waiting_on_session: null,
      open_review_items: [],
    }, null, 2)}\n`,
    "utf8",
  );
  fs.writeFileSync(receiptsPath, "", "utf8");
  fs.writeFileSync(notificationsPath, [
    JSON.stringify({
      schema_version: "wp_notification@1",
      timestamp_utc: "2026-04-08T12:00:00Z",
      wp_id: wpId,
      source_kind: "AUTO_ROUTE",
      source_role: "INTEGRATION_VALIDATOR",
      source_session: "intval-1",
      target_role: "ORCHESTRATOR",
      target_session: null,
      correlation_id: "final-1",
      summary: "AUTO_ROUTE: direct review lane complete; orchestrator verdict progression ready",
    }),
    JSON.stringify({
      schema_version: "wp_notification@1",
      timestamp_utc: "2026-04-08T12:00:01Z",
      wp_id: wpId,
      source_kind: "SESSION_COMPLETION",
      source_role: "CODER",
      source_session: "coder-1",
      target_role: "ORCHESTRATOR",
      target_session: null,
      correlation_id: "close-1",
      summary: "Governed session closed cleanly",
    }),
  ].join("\n") + "\n", "utf8");

  try {
    const active = checkNotifications({ wpId, role: "ORCHESTRATOR" });
    const history = checkNotifications({ wpId, role: "ORCHESTRATOR", history: true });

    assert.equal(active.pendingCount, 0);
    assert.equal(active.historyHidden, true);
    assert.equal(active.hiddenPendingCount, 2);
    assert.deepEqual(active.hiddenByKind, {
      AUTO_ROUTE: 1,
      SESSION_COMPLETION: 1,
    });

    assert.equal(history.pendingCount, 2);
    assert.equal(history.historyHidden, false);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});
