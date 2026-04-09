import assert from "node:assert/strict";
import test from "node:test";

import {
  entryMatchesRoleContext,
  entryMatchesTriggerContext,
  resolveRecallContext,
  scoreMemoryForRecall,
} from "../scripts/memory/memory-recall.mjs";

test("resolveRecallContext derives default steering trigger and script hints", () => {
  const context = resolveRecallContext("STEERING", { wpId: "WP-TEST-v1" });

  assert.deepEqual(context.roleCandidates, ["ORCHESTRATOR"]);
  assert.deepEqual(context.triggerRefs, ["orchestrator-steer-next"]);
  assert.ok(context.scriptCandidates.includes("orchestrator-steer-next.mjs"));
  assert.equal(context.primaryRole, "ORCHESTRATOR");
  assert.equal(context.primaryTrigger, "orchestrator-steer-next");
});

test("resolveRecallContext keeps RESUME neutral without a WP and targets orchestrator-next when scoped", () => {
  const neutralContext = resolveRecallContext("RESUME", {});
  assert.deepEqual(neutralContext.roleCandidates, []);
  assert.deepEqual(neutralContext.triggerRefs, []);

  const wpContext = resolveRecallContext("RESUME", { wpId: "WP-TEST-v1" });
  assert.deepEqual(wpContext.roleCandidates, ["ORCHESTRATOR"]);
  assert.deepEqual(wpContext.triggerRefs, ["orchestrator-next"]);
  assert.ok(wpContext.scriptCandidates.includes("orchestrator-next.mjs"));
});

test("entryMatchesTriggerContext matches fail-capture script metadata", () => {
  const context = resolveRecallContext("STEERING", { wpId: "WP-TEST-v1" });
  const entry = {
    topic: "Script failure: orchestrator-steer-next.mjs - packet drift",
    summary: "Script orchestrator-steer-next.mjs failed on packet drift",
    content: "Script orchestrator-steer-next.mjs failed: packet drift",
    source_artifact: "fail-capture",
    metadata: JSON.stringify({ script: "orchestrator-steer-next.mjs" }),
  };

  assert.equal(entryMatchesTriggerContext(entry, context), true);
});

test("entryMatchesRoleContext accepts role-authored habit memories", () => {
  const context = resolveRecallContext("CODER_RESUME", { wpId: "WP-TEST-v1" });
  const entry = {
    memory_type: "procedural",
    source_artifact: "memory-capture",
    source_role: "CODER",
    metadata: JSON.stringify({ captured_mid_session: true }),
  };

  assert.equal(entryMatchesRoleContext(entry, context), true);
});

test("scoreMemoryForRecall gives trigger-matched failures priority over generic memories", () => {
  const context = resolveRecallContext("STEERING", { wpId: "WP-TEST-v1" });
  const triggerEntry = {
    topic: "Script failure: orchestrator-steer-next.mjs - packet drift",
    summary: "Script orchestrator-steer-next.mjs failed: packet drift",
    content: "Script orchestrator-steer-next.mjs failed: packet drift",
    source_artifact: "fail-capture",
    source_role: "ORCHESTRATOR",
    wp_id: "WP-TEST-v1",
    importance: 0.5,
    access_count: 0,
    metadata: JSON.stringify({ script: "orchestrator-steer-next.mjs" }),
  };
  const genericEntry = {
    topic: "General memory",
    summary: "General memory",
    content: "General memory",
    source_artifact: "conversation-promotion",
    source_role: "",
    wp_id: "",
    importance: 0.9,
    access_count: 0,
    metadata: "{}",
  };

  assert.ok(scoreMemoryForRecall(triggerEntry, context) > scoreMemoryForRecall(genericEntry, context));
});
