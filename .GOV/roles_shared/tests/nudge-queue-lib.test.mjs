import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  drainNudges,
  enqueueNudge,
  listQueueDepth,
  NUDGE_MAX_QUEUE_DEPTH,
  nudgeQueueDir,
} from "../scripts/session/nudge-queue-lib.mjs";

function tempRoot() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "hsk-nudge-queue-"));
}

function payload(overrides = {}) {
  return {
    kind: "STEER",
    from_role: "ORCHESTRATOR",
    wp_id: "WP-TEST-NUDGE-v1",
    correlation_id: `nudge-${Math.random().toString(16).slice(2)}`,
    body: { message: "test nudge" },
    ...overrides,
  };
}

test("enqueueNudge writes validated payloads and reports queue depth", () => {
  const root = tempRoot();
  try {
    const result = enqueueNudge({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      payload: payload({ correlation_id: "one" }),
      runtimeRootAbs: root,
    });
    assert.equal(result.ok, true);
    assert.equal(result.queueDepth, 1);
    assert.equal(listQueueDepth("CODER:WP-TEST-NUDGE-v1", { runtimeRootAbs: root }), 1);
    assert.equal(fs.existsSync(result.filePath), true);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("drainNudges claims FIFO files exactly once across racing drainers", () => {
  const root = tempRoot();
  try {
    enqueueNudge({ sessionId: "CODER:WP-TEST-NUDGE-v1", payload: payload({ correlation_id: "first" }), runtimeRootAbs: root });
    enqueueNudge({ sessionId: "CODER:WP-TEST-NUDGE-v1", payload: payload({ correlation_id: "second" }), runtimeRootAbs: root });

    const firstDrain = drainNudges({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      drainerId: "drainer-a",
      runtimeRootAbs: root,
    });
    const secondDrain = drainNudges({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      drainerId: "drainer-b",
      runtimeRootAbs: root,
    });

    assert.deepEqual(firstDrain.nudges.map((entry) => entry.correlation_id), ["first", "second"]);
    assert.deepEqual(secondDrain.nudges, []);
    assert.equal(listQueueDepth("CODER:WP-TEST-NUDGE-v1", { runtimeRootAbs: root }), 0);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("expirePastTtl removes stale nudges during depth and drain", () => {
  const root = tempRoot();
  try {
    enqueueNudge({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      payload: payload({
        correlation_id: "expired",
        enqueued_at: "2026-01-01T00:00:00.000Z",
        expires_at: "2026-01-01T00:00:01.000Z",
      }),
      runtimeRootAbs: root,
    });
    assert.equal(listQueueDepth("CODER:WP-TEST-NUDGE-v1", {
      runtimeRootAbs: root,
    }), 0);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("enqueueNudge rejects queue depth overflow explicitly", () => {
  const root = tempRoot();
  try {
    for (let i = 0; i < NUDGE_MAX_QUEUE_DEPTH; i += 1) {
      const result = enqueueNudge({
        sessionId: "CODER:WP-TEST-NUDGE-v1",
        payload: payload({ correlation_id: `fill-${i}` }),
        runtimeRootAbs: root,
      });
      assert.equal(result.ok, true);
    }
    const overflow = enqueueNudge({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      payload: payload({ correlation_id: "overflow" }),
      runtimeRootAbs: root,
    });
    assert.equal(overflow.ok, false);
    assert.match(overflow.error, /depth cap/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("drainNudges requeues failed delivery with attempts preserved", () => {
  const root = tempRoot();
  try {
    enqueueNudge({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      payload: payload({ correlation_id: "retry-me" }),
      runtimeRootAbs: root,
    });
    const failed = drainNudges({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      drainerId: "drainer-fail",
      runtimeRootAbs: root,
      deliver() {
        throw new Error("simulated delivery failure");
      },
    });
    assert.equal(failed.nudges.length, 0);
    assert.equal(failed.failed.length, 1);
    assert.equal(listQueueDepth("CODER:WP-TEST-NUDGE-v1", { runtimeRootAbs: root }), 1);

    const retried = drainNudges({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      drainerId: "drainer-success",
      runtimeRootAbs: root,
    });
    assert.equal(retried.nudges[0].correlation_id, "retry-me");
    assert.equal(retried.nudges[0].delivery_attempts, 2);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("drainNudges recovers stale claimed files without deleting the nudge", () => {
  const root = tempRoot();
  try {
    const enqueued = enqueueNudge({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      payload: payload({ correlation_id: "orphaned" }),
      runtimeRootAbs: root,
    });
    const claimedPath = enqueued.filePath.replace(/\.json$/, ".claimed");
    fs.renameSync(enqueued.filePath, claimedPath);
    const old = new Date(Date.now() - 10 * 60 * 1000);
    fs.utimesSync(claimedPath, old, old);

    const drained = drainNudges({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      drainerId: "drainer-recover",
      runtimeRootAbs: root,
    });
    assert.equal(drained.orphansRecovered, 1);
    assert.equal(drained.nudges[0].correlation_id, "orphaned");

    const dir = nudgeQueueDir({
      sessionId: "CODER:WP-TEST-NUDGE-v1",
      wpId: "WP-TEST-NUDGE-v1",
      runtimeRootAbs: root,
    });
    assert.deepEqual(fs.readdirSync(dir), []);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
