import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const CHECK_SCRIPT = fileURLToPath(new URL("./hbr-matrix-check.mjs", import.meta.url));

function withTempDir(fn) {
  const root = mkdtempSync(path.join(tmpdir(), "hbr-matrix-check-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function writePacket(root, relativePath, packet) {
  const packetPath = path.join(root, relativePath);
  mkdirSync(path.dirname(packetPath), { recursive: true });
  writeFileSync(packetPath, JSON.stringify(packet, null, 2));
  return packetPath;
}

function packetWithMatrix(hbr, hbrNotApplicable = []) {
  return {
    acceptance_matrix: {
      hbr,
      hbr_not_applicable: hbrNotApplicable,
    },
  };
}

function writeActiveHbrRegistry(root) {
  const registryPath = path.join(root, ".GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json");
  mkdirSync(path.dirname(registryPath), { recursive: true });
  writeFileSync(registryPath, JSON.stringify({
    status: "ACTIVE",
    enforcement: {
      implementation_status: "ACTIVE",
    },
  }, null, 2));
  return registryPath;
}

function runCheck(args, cwd) {
  return spawnSync(process.execPath, [CHECK_SCRIPT, ...args], {
    cwd,
    encoding: "utf8",
  });
}

function parseJsonLines(stderr) {
  return stderr.trim().split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
}

test("--packet passes a golden PROVED HBR row", () => withTempDir((root) => {
  const packetPath = writePacket(root, "packet.json", packetWithMatrix([
    {
      hbr_id: "HBR-001",
      status: "PROVED",
      evidence_pointer: "evidence/HBR-001.md",
      validator_verdict: "PROVED",
    },
  ]));

  const result = runCheck(["--packet", packetPath], root);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /hbr-matrix-check ok/);
}));

test("PENDING HBR row exits 2 and emits JSONL failure", () => withTempDir((root) => {
  const packetPath = writePacket(root, "packet.json", packetWithMatrix([
    {
      hbr_id: "HBR-002",
      status: "PENDING",
    },
  ]));

  const result = runCheck(["--packet", packetPath], root);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].hbr_id, "HBR-002");
  assert.equal(failures[0].packet, packetPath);
  assert.match(failures[0].reason, /PENDING/);
  assert.ok(failures[0].severity);
}));

test("PROVED HBR row without evidence_pointer exits 2", () => withTempDir((root) => {
  const packetPath = writePacket(root, "packet.json", packetWithMatrix([
    {
      hbr_id: "HBR-003",
      status: "PROVED",
      evidence_pointer: "",
      validator_verdict: "PROVED",
    },
  ]));

  const result = runCheck(["--packet", packetPath], root);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].hbr_id, "HBR-003");
  assert.match(failures[0].reason, /evidence_pointer/);
}));

test("NOT_APPLICABLE HBR row without ledger reason exits 2", () => withTempDir((root) => {
  const packetPath = writePacket(root, "packet.json", packetWithMatrix([
    {
      hbr_id: "HBR-004",
      status: "NOT_APPLICABLE",
    },
  ]));

  const result = runCheck(["--packet", packetPath], root);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].hbr_id, "HBR-004");
  assert.match(failures[0].reason, /hbr_not_applicable/);
}));

test("HBR-QUIET-004 row requires packet requires_foreground true", () => withTempDir((root) => {
  const foregroundRow = {
    hbr_id: "HBR-QUIET-004",
    status: "PROVED",
    evidence_pointer: "evidence/HBR-QUIET-004.json",
    validator_verdict: "PROVED",
  };
  const packetPath = writePacket(root, "packet.json", packetWithMatrix([foregroundRow]));

  const result = runCheck(["--packet", packetPath], root);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].hbr_id, "HBR-QUIET-004");
  assert.match(failures[0].reason, /requires_foreground/);

  const approvedPath = writePacket(root, "approved-packet.json", {
    requires_foreground: true,
    ...packetWithMatrix([foregroundRow]),
  });
  const approved = runCheck(["--packet", approvedPath], root);
  assert.equal(approved.status, 0, `stdout:\n${approved.stdout}\nstderr:\n${approved.stderr}`);
}));

test("packet OPEN_BLOCKERS entries block closure even when HBR rows are proved", () => withTempDir((root) => {
  const packetPath = writePacket(root, "packet.json", {
    open_blockers: [
      {
        blocker_id: "hbr-vis-gap-19ae4e9c4f2d",
        blocker_kind: "HBR_VIS_GAP",
        status: "OPEN",
        hbr_id: "HBR-VIS-005",
        surface_name: "Diagnostics canvas controls",
        surface_path: "app://diagnostics/canvas-controls",
        gap_class: "opaque_canvas",
        receipt_uuid: "018f6d3a-1f00-7a2b-8c3d-123456789abc",
      },
    ],
    ...packetWithMatrix([
      {
        hbr_id: "HBR-VIS-005",
        status: "PROVED",
        evidence_pointer: "receipt://018f6d3a-1f00-7a2b-8c3d-123456789abc",
        validator_verdict: "PROVED",
      },
    ]),
  });

  const result = runCheck(["--packet", packetPath], root);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].hbr_id, "HBR-VIS-005");
  assert.equal(failures[0].severity, "OPEN_BLOCKER");
  assert.match(failures[0].reason, /OPEN_BLOCKERS/);
  assert.match(failures[0].reason, /Diagnostics canvas controls/);
}));

test("--all-packets walks .GOV/task_packets/*/packet.json", () => withTempDir((root) => {
  writePacket(root, ".GOV/task_packets/WP-TEST-001/packet.json", packetWithMatrix([
    {
      hbr_id: "HBR-005",
      status: "PROVED",
      evidence_pointer: "evidence/HBR-005.md",
      validator_verdict: "PROVED",
    },
  ]));

  const result = runCheck(["--all-packets"], root);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /1 packet/);
}));

test("--all-packets skips packets without an HBR matrix", () => withTempDir((root) => {
  writePacket(root, ".GOV/task_packets/WP-LEGACY-NO-HBR/packet.json", {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-LEGACY-NO-HBR",
  });
  writePacket(root, ".GOV/task_packets/WP-HBR-READY/packet.json", packetWithMatrix([
    {
      hbr_id: "HBR-006",
      status: "PROVED",
      evidence_pointer: "evidence/HBR-006.md",
      validator_verdict: "PROVED",
    },
  ]));

  const result = runCheck(["--all-packets"], root);

  assert.equal(result.status, 0);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /1 packet/);
}));

test("--all-packets fails closed when active HBR enforcement finds zero matrix packets", () => withTempDir((root) => {
  writeActiveHbrRegistry(root);
  writePacket(root, ".GOV/task_packets/WP-LEGACY-NO-HBR/packet.json", {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-LEGACY-NO-HBR",
  });

  const result = runCheck(["--all-packets"], root);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 2);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].severity, "MATRIX_COVERAGE_GAP");
  assert.match(failures[0].reason, /0 packets/);
}));

test("malformed packet input exits 3", () => withTempDir((root) => {
  const packetPath = path.join(root, "packet.json");
  writeFileSync(packetPath, "{");

  const result = runCheck(["--packet", packetPath], root);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 3);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].packet, packetPath);
  assert.equal(failures[0].severity, "MALFORMED");
}));

test("CLI error exits 3", () => withTempDir((root) => {
  const result = runCheck([], root);
  const failures = parseJsonLines(result.stderr);

  assert.equal(result.status, 3);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].severity, "CLI_ERROR");
}));
