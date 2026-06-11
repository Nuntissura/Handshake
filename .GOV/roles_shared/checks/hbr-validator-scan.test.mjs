import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = fileURLToPath(new URL("./hbr-validator-scan.mjs", import.meta.url));

function withTempRepo(fn) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hbr-validator-scan-"));
  try {
    return fn(root);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function writePacket(root, packet) {
  const packetPath = path.join(root, ".GOV", "task_packets", packet.wp_id, "packet.json");
  writeJson(packetPath, packet);
  return packetPath;
}

function packetWithRows(rows, extraMatrix = {}) {
  const wpId = "WP-HBR-VALIDATOR-SCAN-TEST-v1";
  return {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
    workflow: {
      receipts_file: `gov_runtime/roles_shared/WP_COMMUNICATIONS/${wpId}/RECEIPTS.jsonl`,
    },
    acceptance_matrix: {
      schema_version: 1,
      hbr: rows,
      hbr_not_applicable: [],
      hbr_evidence_results: [],
      ...extraMatrix,
    },
  };
}

function provedRow(hbrId, evidencePointer) {
  return {
    hbr_id: hbrId,
    status: "PROVED",
    evidence_pointer: evidencePointer,
    validator_verdict: "PROVED",
  };
}

function runScan(root, packetPath, extraArgs = []) {
  return spawnSync(process.execPath, [
    SCRIPT_PATH,
    "--packet",
    packetPath,
    "--repo-root",
    root,
    ...extraArgs,
  ], {
    cwd: root,
    encoding: "utf8",
  });
}

test("missing test evidence downgrades claimed PROVED row to STEER and exits 2", () => withTempRepo((root) => {
  const packetPath = writePacket(root, packetWithRows([
    provedRow("HBR-INT-001", "test://missing_hbr_test"),
  ], {
    hbr_evidence_results: [
      { evidence_pointer: "test://missing_hbr_test", status: "PASS" },
    ],
  }));

  const result = runScan(root, packetPath);
  const packet = JSON.parse(fs.readFileSync(packetPath, "utf8"));
  const row = packet.acceptance_matrix.hbr[0];

  assert.equal(result.status, 2);
  assert.match(result.stderr, /missing_hbr_test/);
  assert.equal(row.status, "STEER");
  assert.equal(row.validator_verdict, "STEER");
  assert.match(row.steer_reason, /test evidence not found/);
  assert.equal(row.evidence_verification.status, "STEER");
}));

test("existing passing test evidence keeps PROVED row unchanged", () => withTempRepo((root) => {
  const testPath = path.join(root, "src", "backend", "handshake_core", "tests", "hbr_fixture_tests.rs");
  fs.mkdirSync(path.dirname(testPath), { recursive: true });
  fs.writeFileSync(testPath, "#[test]\nfn hbr_existing_passes() {}\n", "utf8");
  const packetPath = writePacket(root, packetWithRows([
    provedRow("HBR-INT-001", "test://hbr_existing_passes"),
  ], {
    hbr_evidence_results: [
      { evidence_pointer: "test://hbr_existing_passes", status: "PASS" },
    ],
  }));

  const result = runScan(root, packetPath);
  const packet = JSON.parse(fs.readFileSync(packetPath, "utf8"));
  const row = packet.acceptance_matrix.hbr[0];

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
  assert.equal(row.status, "PROVED");
  assert.equal(row.validator_verdict, "PROVED");
  assert.equal(row.steer_reason, undefined);
}));

test("receipt artifact and event evidence pointers resolve from durable artifacts", () => withTempRepo((root) => {
  const wpId = "WP-HBR-VALIDATOR-SCAN-TEST-v1";
  const artifactRel = "evidence/hbr-artifact.json";
  fs.mkdirSync(path.join(root, "evidence"), { recursive: true });
  fs.writeFileSync(path.join(root, artifactRel), "{\"ok\":true}\n", "utf8");

  const receiptsPath = path.join(root, "gov_runtime", "roles_shared", "WP_COMMUNICATIONS", wpId, "RECEIPTS.jsonl");
  fs.mkdirSync(path.dirname(receiptsPath), { recursive: true });
  fs.writeFileSync(receiptsPath, `${JSON.stringify({
    receipt_id: "receipt-hbr-1",
    refs: ["receipt://receipt-hbr-1"],
  })}\n`, "utf8");

  const eventPath = path.join(root, "event-ledger.jsonl");
  fs.writeFileSync(eventPath, `${JSON.stringify({
    event_id: "event-hbr-1",
    event_type: "HBR_HANDOFF_GATE",
  })}\n`, "utf8");

  const packetPath = writePacket(root, packetWithRows([
    provedRow("HBR-MAN-001", `artifact://${artifactRel}`),
    provedRow("HBR-INT-007", "receipt://receipt-hbr-1"),
    provedRow("HBR-INT-008", "event://event-hbr-1"),
  ]));

  const result = runScan(root, packetPath, ["--event-ledger", eventPath]);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
  const packet = JSON.parse(fs.readFileSync(packetPath, "utf8"));
  assert.deepEqual(packet.acceptance_matrix.hbr.map((row) => row.status), [
    "PROVED",
    "PROVED",
    "PROVED",
  ]);
}));

test("unknown evidence URI scheme downgrades with explicit reason", () => withTempRepo((root) => {
  const packetPath = writePacket(root, packetWithRows([
    provedRow("HBR-INT-001", "mystery://proof"),
  ]));

  const result = runScan(root, packetPath);
  const packet = JSON.parse(fs.readFileSync(packetPath, "utf8"));
  const row = packet.acceptance_matrix.hbr[0];

  assert.equal(result.status, 2);
  assert.match(result.stderr, /unknown evidence_pointer scheme/i);
  assert.equal(row.status, "STEER");
  assert.match(row.steer_reason, /unknown evidence_pointer scheme/i);
}));

test("packet without HBR matrix exits 0 because there are no PROVED rows to verify", () => withTempRepo((root) => {
  const packetPath = writePacket(root, {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-HBR-VALIDATOR-SCAN-NO-MATRIX-v1",
  });

  const result = runScan(root, packetPath);
  const packet = JSON.parse(fs.readFileSync(packetPath, "utf8"));

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
  assert.equal(packet.acceptance_matrix, undefined);
}));
