import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs, {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const CHECK_SCRIPT = fileURLToPath(new URL("./kb-ready-checklist-coverage-check.mjs", import.meta.url));
const GOV_CHECK_SCRIPT = fileURLToPath(new URL("./gov-check.mjs", import.meta.url));

function withFixture(fn) {
  const root = mkdtempSync(path.join(os.tmpdir(), "kb-ready-checklist-coverage-"));
  try {
    return fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}

function writePacket(taskPacketsRoot, wpId, packet) {
  const packetDir = path.join(taskPacketsRoot, wpId);
  mkdirSync(packetDir, { recursive: true });
  writeFileSync(path.join(packetDir, "packet.json"), JSON.stringify(packet, null, 2), "utf8");
}

function writeMt(taskPacketsRoot, wpId, mtId, contract) {
  const packetDir = path.join(taskPacketsRoot, wpId);
  mkdirSync(packetDir, { recursive: true });
  writeFileSync(path.join(packetDir, `${mtId}.json`), JSON.stringify(contract, null, 2), "utf8");
}

function appendReceiptLine(commsRoot, wpId, receipt) {
  const wpDir = path.join(commsRoot, wpId);
  mkdirSync(wpDir, { recursive: true });
  const receiptsFile = path.join(wpDir, "KB_READY_CHECKLIST_RECEIPTS.jsonl");
  fs.appendFileSync(receiptsFile, `${JSON.stringify(receipt)}\n`, "utf8");
}

function makeReceipt({
  wpId,
  mtId,
  verdict = "PASS",
  blockers = [],
  actorSession = "TEST",
  generatedAt = new Date().toISOString(),
}) {
  return {
    schema_id: "hsk.kb_ready_checklist_receipt@1",
    schema_version: "kb_ready_checklist_receipt_v1",
    receipt_kind: "KB_READY_CHECKLIST_RECEIPT",
    wp_id: wpId,
    mt_id: mtId,
    actor_role: "KERNEL_BUILDER",
    actor_session: actorSession,
    generated_at_utc: generatedAt,
    summary: "test fixture receipt",
    overall_verdict: verdict,
    blockers,
    rubric_items: [],
  };
}

function readyForValidationMt(mtId, extras = {}) {
  return {
    mt_id: mtId,
    lifecycle: {
      status: "READY_FOR_VALIDATION",
      validator_verdict: "PENDING",
      ...extras,
    },
  };
}

function runCheck(taskPacketsRoot, commsRoot, extraArgs = []) {
  return spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--task-packets-root",
    taskPacketsRoot,
    "--comms-root",
    commsRoot,
    "--json",
    ...extraArgs,
  ], { encoding: "utf8" });
}

function parseJsonStdout(stdout) {
  return JSON.parse(stdout.trim());
}

test("happy path: READY_FOR_VALIDATION MT with PASS receipt → 0 concerns, exit 0", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  writePacket(taskPacketsRoot, "WP-HAPPY", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-HAPPY", "MT-001", readyForValidationMt("MT-001"));
  appendReceiptLine(commsRoot, "WP-HAPPY", makeReceipt({
    wpId: "WP-HAPPY",
    mtId: "MT-001",
    verdict: "PASS",
  }));

  const result = runCheck(taskPacketsRoot, commsRoot);
  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, true);
  assert.equal(json.packets_scanned, 1);
  assert.equal(json.concerns.length, 0);
}));

test("drift: READY_FOR_VALIDATION MT with no receipt → 1 concern, exit 1", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  writePacket(taskPacketsRoot, "WP-NO-RECEIPT", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-NO-RECEIPT", "MT-007", readyForValidationMt("MT-007"));

  const result = runCheck(taskPacketsRoot, commsRoot);
  assert.equal(result.status, 1, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, false);
  assert.equal(json.packets_scanned, 1);
  assert.equal(json.concerns.length, 1);
  const concern = json.concerns[0];
  assert.equal(concern.severity, "MT_READY_FOR_VALIDATION_WITHOUT_KB_READY_RECEIPT");
  assert.equal(concern.wp_id, "WP-NO-RECEIPT");
  assert.equal(concern.mt_id, "MT-007");
  assert.equal(concern.lifecycle_status, "READY_FOR_VALIDATION");
  assert.match(concern.mt_path, /WP-NO-RECEIPT[\/\\]MT-007\.json$/);
  assert.match(concern.expected_receipt_path, /WP-NO-RECEIPT[\/\\]KB_READY_CHECKLIST_RECEIPTS\.jsonl$/);
}));

test("drift: READY_FOR_VALIDATION MT with BLOCKED receipt → 1 concern, exit 1", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  writePacket(taskPacketsRoot, "WP-BLOCKED", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-BLOCKED", "MT-003", readyForValidationMt("MT-003"));
  appendReceiptLine(commsRoot, "WP-BLOCKED", makeReceipt({
    wpId: "WP-BLOCKED",
    mtId: "MT-003",
    verdict: "BLOCKED",
    blockers: ["RC-002-NO-DEAD-CODE: pub item X unreferenced"],
  }));

  const result = runCheck(taskPacketsRoot, commsRoot);
  assert.equal(result.status, 1, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, false);
  assert.equal(json.concerns.length, 1);
  const concern = json.concerns[0];
  assert.equal(concern.severity, "MT_READY_FOR_VALIDATION_WITH_BLOCKED_KB_READY_RECEIPT");
  assert.equal(concern.receipt_verdict, "BLOCKED");
  assert.deepEqual(concern.blockers, ["RC-002-NO-DEAD-CODE: pub item X unreferenced"]);
}));

test("skipped: non-READY_FOR_VALIDATION MT without receipt → no concern", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  writePacket(taskPacketsRoot, "WP-SKIPPED", { scope: { allowed_paths: ["src/**"] } });
  // COMPLETED MT should not require a receipt.
  writeMt(taskPacketsRoot, "WP-SKIPPED", "MT-100", {
    mt_id: "MT-100",
    lifecycle: { status: "COMPLETED", validator_verdict: "PASS" },
  });
  // BLOCKED MT should not require a receipt.
  writeMt(taskPacketsRoot, "WP-SKIPPED", "MT-101", {
    mt_id: "MT-101",
    lifecycle: { status: "BLOCKED", validator_verdict: "PENDING" },
  });
  // CLAIMED MT should not require a receipt.
  writeMt(taskPacketsRoot, "WP-SKIPPED", "MT-102", {
    mt_id: "MT-102",
    lifecycle: { status: "CLAIMED", validator_verdict: "PENDING" },
  });

  const result = runCheck(taskPacketsRoot, commsRoot);
  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, true);
  assert.equal(json.packets_scanned, 1);
  assert.equal(json.concerns.length, 0);
}));

test("multiple receipts: latest line wins (BLOCKED -> PASS)", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  writePacket(taskPacketsRoot, "WP-LATEST-WINS", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-LATEST-WINS", "MT-005", readyForValidationMt("MT-005"));

  // First (older) receipt was BLOCKED.
  appendReceiptLine(commsRoot, "WP-LATEST-WINS", makeReceipt({
    wpId: "WP-LATEST-WINS",
    mtId: "MT-005",
    verdict: "BLOCKED",
    blockers: ["RC-001-NO-STALE-REASONS: stale message"],
    actorSession: "EARLIER",
    generatedAt: "2026-05-01T00:00:00.000Z",
  }));
  // Later receipt is PASS — should supersede.
  appendReceiptLine(commsRoot, "WP-LATEST-WINS", makeReceipt({
    wpId: "WP-LATEST-WINS",
    mtId: "MT-005",
    verdict: "PASS",
    actorSession: "LATER",
    generatedAt: "2026-05-26T00:00:00.000Z",
  }));

  const result = runCheck(taskPacketsRoot, commsRoot);
  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.concerns.length, 0);
}));

test("multiple receipts: latest line wins (PASS -> BLOCKED) → concern", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  writePacket(taskPacketsRoot, "WP-LATEST-REGRESSION", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-LATEST-REGRESSION", "MT-006", readyForValidationMt("MT-006"));

  appendReceiptLine(commsRoot, "WP-LATEST-REGRESSION", makeReceipt({
    wpId: "WP-LATEST-REGRESSION",
    mtId: "MT-006",
    verdict: "PASS",
    actorSession: "EARLIER",
    generatedAt: "2026-05-01T00:00:00.000Z",
  }));
  appendReceiptLine(commsRoot, "WP-LATEST-REGRESSION", makeReceipt({
    wpId: "WP-LATEST-REGRESSION",
    mtId: "MT-006",
    verdict: "BLOCKED",
    blockers: ["regression"],
    actorSession: "LATER",
    generatedAt: "2026-05-26T00:00:00.000Z",
  }));

  const result = runCheck(taskPacketsRoot, commsRoot);
  assert.equal(result.status, 1, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.concerns.length, 1);
  assert.equal(json.concerns[0].severity, "MT_READY_FOR_VALIDATION_WITH_BLOCKED_KB_READY_RECEIPT");
}));

test("--wp filter isolates a single WP", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  // WP-DRIFTY has a missing-receipt drift; WP-CLEAN is fine.
  writePacket(taskPacketsRoot, "WP-DRIFTY", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-DRIFTY", "MT-001", readyForValidationMt("MT-001"));

  writePacket(taskPacketsRoot, "WP-CLEAN", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-CLEAN", "MT-001", readyForValidationMt("MT-001"));
  appendReceiptLine(commsRoot, "WP-CLEAN", makeReceipt({
    wpId: "WP-CLEAN",
    mtId: "MT-001",
    verdict: "PASS",
  }));

  // Scope to clean WP — should be 0 concerns.
  const cleanResult = runCheck(taskPacketsRoot, commsRoot, ["--wp", "WP-CLEAN"]);
  assert.equal(cleanResult.status, 0, `stdout:\n${cleanResult.stdout}\nstderr:\n${cleanResult.stderr}`);
  const cleanJson = parseJsonStdout(cleanResult.stdout);
  assert.equal(cleanJson.packets_scanned, 1);
  assert.equal(cleanJson.concerns.length, 0);

  // Scope to drifty WP — should be 1 concern.
  const driftResult = runCheck(taskPacketsRoot, commsRoot, ["--wp", "WP-DRIFTY"]);
  assert.equal(driftResult.status, 1, `stdout:\n${driftResult.stdout}\nstderr:\n${driftResult.stderr}`);
  const driftJson = parseJsonStdout(driftResult.stdout);
  assert.equal(driftJson.packets_scanned, 1);
  assert.equal(driftJson.concerns.length, 1);
  assert.equal(driftJson.concerns[0].wp_id, "WP-DRIFTY");
}));

test("empty: no packets present yields exit 0 with no concerns", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  const result = runCheck(taskPacketsRoot, commsRoot);
  assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.equal(json.ok, true);
  assert.equal(json.packets_scanned, 0);
  assert.equal(json.concerns.length, 0);
}));

test("text mode: human-readable output prints CONCERNS block on drift", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  writePacket(taskPacketsRoot, "WP-TEXT-DRIFT", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-TEXT-DRIFT", "MT-042", readyForValidationMt("MT-042"));

  const result = spawnSync(process.execPath, [
    CHECK_SCRIPT,
    "--task-packets-root",
    taskPacketsRoot,
    "--comms-root",
    commsRoot,
  ], { encoding: "utf8" });

  assert.equal(result.status, 1, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  assert.match(result.stdout, /kb-ready-checklist-coverage-check: scanned 1 packet/);
  assert.match(result.stdout, /CONCERNS: 1/);
  assert.match(result.stdout, /WP-TEXT-DRIFT: 1 concern/);
  assert.match(result.stdout, /MT-042 \(MT_READY_FOR_VALIDATION_WITHOUT_KB_READY_RECEIPT\)/);
}));

test("malformed receipt line: surfaces as read_error and exit 2", () => withFixture((root) => {
  const taskPacketsRoot = path.join(root, "task_packets");
  const commsRoot = path.join(root, "comms");
  mkdirSync(taskPacketsRoot, { recursive: true });
  mkdirSync(commsRoot, { recursive: true });

  writePacket(taskPacketsRoot, "WP-MALFORMED-RECEIPT", { scope: { allowed_paths: ["src/**"] } });
  writeMt(taskPacketsRoot, "WP-MALFORMED-RECEIPT", "MT-001", readyForValidationMt("MT-001"));
  const wpDir = path.join(commsRoot, "WP-MALFORMED-RECEIPT");
  mkdirSync(wpDir, { recursive: true });
  fs.writeFileSync(
    path.join(wpDir, "KB_READY_CHECKLIST_RECEIPTS.jsonl"),
    "{not valid json\n",
    "utf8",
  );

  const result = runCheck(taskPacketsRoot, commsRoot);
  assert.equal(result.status, 2, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  const json = parseJsonStdout(result.stdout);
  assert.ok(json.read_errors.length >= 1);
}));

test("check is wired into the gov-check bundle", () => {
  const govCheck = readFileSync(GOV_CHECK_SCRIPT, "utf8");
  assert.match(
    govCheck,
    /\["kb-ready-checklist-coverage-check", "\.\/kb-ready-checklist-coverage-check\.mjs", "WORK_PACKET"\]/,
  );
});
