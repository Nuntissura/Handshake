import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  emptyRegistry,
  findEntry,
  loadRegistryJson,
  parseLegacyMarkdown,
  persistRegistry,
  renderRegistryMarkdown,
  REGISTRY_SCHEMA_ID,
  upsertEntry,
  writeRegistryJson,
} from "../scripts/lib/traceability-registry-lib.mjs";

function tempPaths() {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "trace-registry-"));
  return {
    tempRoot,
    jsonPath: path.join(tempRoot, "wp_traceability_registry.json"),
    mdPath: path.join(tempRoot, "WP_TRACEABILITY_REGISTRY.md"),
    cleanup: () => fs.rmSync(tempRoot, { recursive: true, force: true }),
  };
}

test("emptyRegistry produces a contract with required fields", () => {
  const reg = emptyRegistry();
  assert.equal(reg.schema_id, REGISTRY_SCHEMA_ID);
  assert.equal(reg.authority_surface, "MACHINE_CONTRACT");
  assert.equal(reg.rendered_md_projection_policy, "GENERATED_FROM_JSON_DO_NOT_HAND_EDIT");
  assert.deepEqual(reg.entries, []);
  assert.deepEqual(reg.superseded_activation_history, []);
  assert.deepEqual(reg.historical_lineage, []);
});

test("writeRegistryJson + loadRegistryJson round-trips a populated registry", () => {
  const { jsonPath, cleanup } = tempPaths();
  try {
    const reg = emptyRegistry();
    upsertEntry(reg, {
      base_wp_id: "WP-1-Test",
      active_packet_path: ".GOV/task_packets/stubs/WP-1-Test-v1.contract.json",
      task_board_projection: "Stub Backlog (Not Activated): WP-1-Test-v1",
      notes: "test entry",
    });
    writeRegistryJson(jsonPath, reg);
    const loaded = loadRegistryJson(jsonPath);
    assert.equal(loaded.entries.length, 1);
    assert.equal(loaded.entries[0].base_wp_id, "WP-1-Test");
    assert.equal(loaded.entries[0].notes, "test entry");
  } finally {
    cleanup();
  }
});

test("loadRegistryJson rejects wrong schema_id", () => {
  const { jsonPath, cleanup } = tempPaths();
  try {
    fs.writeFileSync(jsonPath, JSON.stringify({ schema_id: "wrong", entries: [] }), "utf8");
    assert.throws(() => loadRegistryJson(jsonPath), /Unexpected schema_id/);
  } finally {
    cleanup();
  }
});

test("upsertEntry inserts a new row when base_wp_id is new", () => {
  const reg = emptyRegistry();
  upsertEntry(reg, { base_wp_id: "WP-1-A", active_packet_path: "p" });
  assert.equal(reg.entries.length, 1);
  assert.equal(reg.entries[0].base_wp_id, "WP-1-A");
  assert.equal(reg.entries[0].task_board_projection, "TBD");
});

test("upsertEntry preserves existing task_board_projection + notes when only path changes", () => {
  const reg = emptyRegistry();
  upsertEntry(reg, {
    base_wp_id: "WP-1-A",
    active_packet_path: "old.md",
    task_board_projection: "In Progress: WP-1-A-v1",
    notes: "first revision",
  });
  upsertEntry(reg, {
    base_wp_id: "WP-1-A",
    active_packet_path: "new.md",
  });
  assert.equal(reg.entries.length, 1);
  assert.equal(reg.entries[0].active_packet_path, "new.md");
  assert.equal(reg.entries[0].task_board_projection, "In Progress: WP-1-A-v1");
  assert.equal(reg.entries[0].notes, "first revision");
});

test("upsertEntry can override task_board_projection + notes when explicitly provided", () => {
  const reg = emptyRegistry();
  upsertEntry(reg, {
    base_wp_id: "WP-1-A",
    active_packet_path: "old.md",
    task_board_projection: "Ready for Dev: WP-1-A-v1",
    notes: "ready",
  });
  upsertEntry(reg, {
    base_wp_id: "WP-1-A",
    active_packet_path: "v2.md",
    task_board_projection: "Done: WP-1-A-v2",
    notes: "promoted",
  });
  assert.equal(reg.entries[0].task_board_projection, "Done: WP-1-A-v2");
  assert.equal(reg.entries[0].notes, "promoted");
});

test("findEntry returns null for missing base_wp_id", () => {
  const reg = emptyRegistry();
  upsertEntry(reg, { base_wp_id: "WP-1-A", active_packet_path: "p" });
  assert.equal(findEntry(reg, "WP-1-A").base_wp_id, "WP-1-A");
  assert.equal(findEntry(reg, "WP-1-MISSING"), null);
});

test("parseLegacyMarkdown handles all three table sections", () => {
  const md = [
    "# Header",
    "## Registry",
    "| Base WP ID | Active Packet | Task Board | Notes |",
    "|-----------:|---------------|------------|-------|",
    "| WP-1-First | path1.md | Ready for Dev: WP-1-First | first |",
    "| WP-1-Second | path2.md | Done: WP-1-Second | second |",
    "",
    "## Superseded Activation History",
    "",
    "| BASE_WP_ID | SUPERSEDED_PACKET_OR_STUB | TASK_BOARD | Replacement / Notes |",
    "|------------|---------------------------|------------|---------------------|",
    "",
    "| WP-1-Old | path-old.md | Superseded (Archive): WP-1-Old | folded into WP-1-First |",
    "",
    "## Historical Failure + Live Smoketest Lineage",
    "| Base WP ID | Historical Failed Packet | Historical Classification | Live Smoketest Status | Active Recovery Packet | Driver Audit | Latest Smoketest Review |",
    "|-----------:|--------------------------|---------------------------|----------------------|------------------------|--------------|-------------------------|",
    "| WP-1-Lin | WP-1-Lin-v3 | FAILED_HISTORICAL_CLOSURE | LIVE_SMOKETEST_BASELINE_RECOVERED | WP-1-Lin-v4 | AUDIT-1 | SMOKE-1 |",
  ].join("\n");

  const parsed = parseLegacyMarkdown(md);
  assert.equal(parsed.entries.length, 2);
  assert.equal(parsed.entries[0].base_wp_id, "WP-1-First");
  assert.equal(parsed.entries[1].base_wp_id, "WP-1-Second");
  assert.equal(parsed.supersededActivationHistory.length, 1);
  assert.equal(parsed.supersededActivationHistory[0].base_wp_id, "WP-1-Old");
  assert.equal(parsed.historicalLineage.length, 1);
  assert.equal(parsed.historicalLineage[0].base_wp_id, "WP-1-Lin");
  assert.equal(parsed.historicalLineage[0].active_recovery_packet, "WP-1-Lin-v4");
});

test("parseLegacyMarkdown does not bleed entries across section boundaries", () => {
  const md = [
    "| Base WP ID | Active Packet | Task Board | Notes |",
    "|-----------:|---------------|------------|-------|",
    "| WP-1-Main | p.md | T | n |",
    "## Some Other Section",
    "| WP-1-NotInTable | should-not-appear | T | n |",
  ].join("\n");
  const parsed = parseLegacyMarkdown(md);
  assert.equal(parsed.entries.length, 1);
  assert.equal(parsed.entries[0].base_wp_id, "WP-1-Main");
});

test("persistRegistry writes JSON + regenerates MD and round-trips through render", () => {
  const { jsonPath, mdPath, cleanup } = tempPaths();
  try {
    const reg = emptyRegistry();
    upsertEntry(reg, {
      base_wp_id: "WP-KERNEL-006-Principal-Authority-Foundation",
      active_packet_path:
        ".GOV/task_packets/stubs/WP-KERNEL-006-Principal-Authority-Foundation-v1.contract.json",
      task_board_projection: "TBD",
      notes: "",
    });
    persistRegistry(reg, { jsonPath, mdPath });

    const loaded = loadRegistryJson(jsonPath);
    const md = fs.readFileSync(mdPath, "utf8");
    assert.equal(loaded.entries.length, 1);
    assert.match(md, /WP-KERNEL-006-Principal-Authority-Foundation/);
    assert.match(md, /GENERATED PROJECTION/);

    const expected = renderRegistryMarkdown(loaded);
    assert.equal(md.replace(/\r\n/g, "\n"), expected.replace(/\r\n/g, "\n"));
  } finally {
    cleanup();
  }
});
