import assert from "node:assert/strict";
import test from "node:test";
import { validateHistoricalSmoketestLineage } from "../scripts/lib/historical-smoketest-lineage-lib.mjs";

function buildRegistry({
  schemaTaskBoardProjection = "Done: WP-1-Structured-Collaboration-Schema-Registry-v4",
  loomTaskBoardProjection = "Stub Backlog (Not Activated): WP-1-Loom-Storage-Portability-v4",
  schemaLatestReview = "SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4",
} = {}) {
  return [
    "# Work Packet Traceability Registry (SSoT)",
    "",
    "## Registry (Phase 1)",
    "",
    "| Base WP ID | Active Packet | Task Board | Notes |",
    "|-----------:|---------------|------------|-------|",
    `| WP-1-Structured-Collaboration-Schema-Registry | .GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v4/packet.md | ${schemaTaskBoardProjection} | active=WP-1-Structured-Collaboration-Schema-Registry-v4; historical failure/live smoketest lineage is modeled below; supersedes historical packets/stubs: WP-1-Structured-Collaboration-Schema-Registry-v3 |`,
    `| WP-1-Loom-Storage-Portability | .GOV/task_packets/stubs/WP-1-Loom-Storage-Portability-v4.md | ${loomTaskBoardProjection} | active=WP-1-Loom-Storage-Portability-v4; historical failure/live smoketest lineage is modeled below; supersedes historical packets/stubs: WP-1-Loom-Storage-Portability-v3 |`,
    "",
    "## Historical Failure + Live Smoketest Lineage",
    "",
    "| Base WP ID | Historical Failed Packet | Historical Classification | Live Smoketest Status | Active Recovery Packet | Driver Audit | Latest Smoketest Review |",
    "|-----------:|--------------------------|---------------------------|----------------------|------------------------|--------------|-------------------------|",
    `| WP-1-Structured-Collaboration-Schema-Registry | WP-1-Structured-Collaboration-Schema-Registry-v3 | FAILED_HISTORICAL_CLOSURE | LIVE_SMOKETEST_BASELINE_RECOVERED | WP-1-Structured-Collaboration-Schema-Registry-v4 | AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT | ${schemaLatestReview} |`,
    "| WP-1-Loom-Storage-Portability | WP-1-Loom-Storage-Portability-v3 | FAILED_HISTORICAL_CLOSURE | LIVE_SMOKETEST_BASELINE_PENDING | WP-1-Loom-Storage-Portability-v4 | AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT | NONE |",
    "",
  ].join("\n");
}

function buildTaskBoard({
  schemaLiveStatus = "LIVE_SMOKETEST_BASELINE_RECOVERED",
} = {}) {
  return [
    "# Handshake Project Task Board",
    "",
    "## Historical Failed Closures Used As Live Smoketest Baselines",
    "",
    `- **[WP-1-Structured-Collaboration-Schema-Registry-v3]** - [FAILED_HISTORICAL_SMOKETEST_BASELINE] - base_wp_id: WP-1-Structured-Collaboration-Schema-Registry - active_recovery: WP-1-Structured-Collaboration-Schema-Registry-v4 - live_status: ${schemaLiveStatus}`,
    "- **[WP-1-Loom-Storage-Portability-v3]** - [FAILED_HISTORICAL_SMOKETEST_BASELINE] - base_wp_id: WP-1-Loom-Storage-Portability - active_recovery: WP-1-Loom-Storage-Portability-v4 - live_status: LIVE_SMOKETEST_BASELINE_PENDING",
    "",
    "## Superseded (Archive)",
    "- **[WP-1-Structured-Collaboration-Schema-Registry-v3]** - [SUPERSEDED]",
    "- **[WP-1-Loom-Storage-Portability-v3]** - [SUPERSEDED]",
    "",
  ].join("\n");
}

test("historical smoketest lineage passes when registry and task board agree", () => {
  const result = validateHistoricalSmoketestLineage({
    registryText: buildRegistry(),
    taskBoardText: buildTaskBoard(),
  });
  assert.deepEqual(result.errors, []);
});

test("historical smoketest lineage fails when a blocked historical row lacks explicit lineage modeling", () => {
  const result = validateHistoricalSmoketestLineage({
    registryText: buildRegistry().replace(/\n## Historical Failure \+ Live Smoketest Lineage[\s\S]*$/, "\n"),
    taskBoardText: buildTaskBoard(),
  });
  assert.match(result.errors.join("\n"), /missing a row in ## Historical Failure \+ Live Smoketest Lineage/i);
});

test("historical smoketest lineage fails when recovered lineage lacks a smoketest review id", () => {
  const result = validateHistoricalSmoketestLineage({
    registryText: buildRegistry({ schemaLatestReview: "NONE" }),
    taskBoardText: buildTaskBoard(),
  });
  assert.match(result.errors.join("\n"), /recovered live smoketest lineage must record LATEST_SMOKETEST_REVIEW/i);
});
