import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  GAP_CLASSES,
  buildHbrVisGap,
  canonicalJsonLine,
  validateHbrVisGap,
} from "./hbr-vis-gap-emit.mjs";

const SCRIPT_PATH = fileURLToPath(new URL("./hbr-vis-gap-emit.mjs", import.meta.url));
const EXPECTED_CANONICAL = "{\"emitted_at_utc\":\"2026-05-18T00:00:00Z\",\"evidence_pointer\":\"artifact://visual/diagnostics-canvas.png\",\"gap_class\":\"opaque_canvas\",\"hbr_id\":\"HBR-VIS-005\",\"proposed_followup_wp\":\"WP-KERNEL-004-VIS-GAP-FOLLOWUP-v1\",\"receipt_kind\":\"HBR_VIS_GAP\",\"receipt_uuid\":\"018f6d3a-1f00-7a2b-8c3d-123456789abc\",\"schema_version\":1,\"surface_name\":\"Diagnostics canvas controls\",\"surface_path\":\"app://diagnostics/canvas-controls\",\"wp_id\":\"WP-KERNEL-004-TEST\"}\n";
const EXPECTED_REQUIRED_ACTION = "Remediate the missing Argus visibility/identification/steering/re-observation path in the same MT/WP when it blocks proof; otherwise record this HBR-VIS gap as a blocker with exact surface, missing Argus capability, affected proof, and recommended remediation before PASS closure.";

function fixtureGap(overrides = {}) {
  return buildHbrVisGap({
    receipt_uuid: "018f6d3a-1f00-7a2b-8c3d-123456789abc",
    wp_id: "WP-KERNEL-004-TEST",
    surface_name: "Diagnostics canvas controls",
    surface_path: "app://diagnostics/canvas-controls",
    gap_class: "opaque_canvas",
    proposed_followup_wp: "WP-KERNEL-004-VIS-GAP-FOLLOWUP-v1",
    evidence_pointer: "artifact://visual/diagnostics-canvas.png",
    emitted_at_utc: "2026-05-18T00:00:00Z",
    ...overrides,
  });
}

function withTempDir(fn) {
  const root = mkdtempSync(path.join(tmpdir(), "hbr-vis-gap-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function writeJson(filePath, value) {
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, JSON.stringify(value, null, 2));
}

test("Node emitter outputs canonical HBR_VIS_GAP JSONL", () => {
  const gap = fixtureGap();

  assert.deepEqual(validateHbrVisGap(gap), []);
  assert.equal(canonicalJsonLine(gap), EXPECTED_CANONICAL);
});

test("all HBR_VIS_GAP gap classes validate", () => {
  for (const gapClass of GAP_CLASSES) {
    assert.deepEqual(validateHbrVisGap(fixtureGap({ gap_class: gapClass })), []);
  }
});

test("CLI appends canonical receipt and structured packet OPEN_BLOCKERS entry", () => withTempDir((root) => {
  const wpId = "WP-KERNEL-004-TEST";
  const packetPath = path.join(root, ".GOV", "task_packets", wpId, "packet.json");
  const receiptPath = path.join(root, "receipts", "RECEIPTS.jsonl");
  writeJson(packetPath, {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
    workflow: {
      receipts_file: "receipts/RECEIPTS.jsonl",
    },
    acceptance_matrix: {
      hbr: [],
      hbr_not_applicable: [],
    },
  });

  const result = spawnSync(process.execPath, [
    SCRIPT_PATH,
    "--wp", wpId,
    "--surface", "Diagnostics canvas controls",
    "--surface-path", "app://diagnostics/canvas-controls",
    "--gap-class", "opaque_canvas",
    "--proposed-followup-wp", "WP-KERNEL-004-VIS-GAP-FOLLOWUP-v1",
    "--evidence-pointer", "artifact://visual/diagnostics-canvas.png",
    "--packet", packetPath,
    "--receipt-uuid", "018f6d3a-1f00-7a2b-8c3d-123456789abc",
    "--emitted-at-utc", "2026-05-18T00:00:00Z",
  ], {
    cwd: root,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.equal(readFileSync(receiptPath, "utf8"), EXPECTED_CANONICAL);

  const packet = JSON.parse(readFileSync(packetPath, "utf8"));
  assert.equal(packet.open_blockers.length, 1);
  assert.deepEqual(packet.open_blockers[0], {
    blocker_id: "hbr-vis-gap-856b3fceea5a",
    blocker_kind: "HBR_VIS_GAP",
    status: "OPEN",
    hbr_id: "HBR-VIS-005",
    wp_id: "WP-KERNEL-004-TEST",
    surface_name: "Diagnostics canvas controls",
    surface_path: "app://diagnostics/canvas-controls",
    gap_class: "opaque_canvas",
    receipt_uuid: "018f6d3a-1f00-7a2b-8c3d-123456789abc",
    receipt_ref: "receipt://018f6d3a-1f00-7a2b-8c3d-123456789abc",
    evidence_pointer: "artifact://visual/diagnostics-canvas.png",
    proposed_followup_wp: "WP-KERNEL-004-VIS-GAP-FOLLOWUP-v1",
    created_at_utc: "2026-05-18T00:00:00Z",
    required_action: EXPECTED_REQUIRED_ACTION,
  });
}));
