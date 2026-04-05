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
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] next_actor=CODER/);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] next_session=coder-test/);
    assert.match(result.stdout, /\[MANUAL_RELAY_NEXT\] dispatch_action=START_SESSION/);
    assert.match(result.stdout, /just manual-relay-dispatch/);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});
