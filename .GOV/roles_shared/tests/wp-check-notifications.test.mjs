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
