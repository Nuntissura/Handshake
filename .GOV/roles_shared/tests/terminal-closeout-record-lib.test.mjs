import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  buildTerminalCloseoutRecordFromCloseoutSync,
  publishTerminalCloseoutRecord,
  readTerminalCloseoutRecord,
  resolveTerminalCloseoutPublication,
  terminalCloseoutStateRank,
} from "../scripts/lib/terminal-closeout-record-lib.mjs";

function tempRecordPath(name) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-terminal-closeout-"));
  return {
    dir,
    file: path.join(dir, `${name}.json`),
  };
}

test("terminal closeout records normalize terminal states and product/debt split", () => {
  const record = buildTerminalCloseoutRecordFromCloseoutSync({
    wpId: "WP-TEST-TERMINAL-RECORD-v1",
    mode: "FAIL",
    packetStatus: "Validated (FAIL)",
    taskBoardStatus: "DONE_FAIL",
    mainContainmentStatus: "NOT_REQUIRED",
    verdict: "FAIL",
    governanceDebtKeys: ["ACTIVE_TOPOLOGY_ARTIFACT_HYGIENE"],
    governanceDebtSummaries: ["Artifact hygiene remains settlement debt."],
    actorRole: "INTEGRATION_VALIDATOR",
    actorSession: "intv:test",
    recordedAtUtc: "2026-04-30T10:00:00Z",
  });

  assert.equal(record.schema_version, "terminal_closeout_record@1");
  assert.equal(record.terminal_state, "SETTLEMENT_DEBT");
  assert.equal(record.product_outcome_state, "FAIL");
  assert.deepEqual(record.governance_debt_keys, ["ACTIVE_TOPOLOGY_ARTIFACT_HYGIENE"]);
  assert.equal(terminalCloseoutStateRank(record.terminal_state), 2);
});

test("terminal closeout writer publishes atomically and rejects downgrades", () => {
  const { dir, file } = tempRecordPath("downgrade");
  try {
    const settled = buildTerminalCloseoutRecordFromCloseoutSync({
      wpId: "WP-TEST-TERMINAL-RECORD-v2",
      mode: "CONTAINED_IN_MAIN",
      packetStatus: "Validated (PASS)",
      taskBoardStatus: "DONE_VALIDATED",
      mainContainmentStatus: "CONTAINED_IN_MAIN",
      mergedMainCommit: "0123456789abcdef0123456789abcdef01234567",
      verdict: "PASS",
      actorRole: "INTEGRATION_VALIDATOR",
      actorSession: "intv:test",
      recordedAtUtc: "2026-04-30T10:10:00Z",
    });
    const published = publishTerminalCloseoutRecord({
      wpId: "WP-TEST-TERMINAL-RECORD-v2",
      record: settled,
      recordPathAbs: file,
    });
    assert.equal(published.record.terminal_state, "TERMINAL_SETTLED");

    const weaker = buildTerminalCloseoutRecordFromCloseoutSync({
      wpId: "WP-TEST-TERMINAL-RECORD-v2",
      mode: "MERGE_PENDING",
      packetStatus: "Done",
      taskBoardStatus: "DONE_MERGE_PENDING",
      mainContainmentStatus: "MERGE_PENDING",
      verdict: "PASS",
      actorRole: "ORCHESTRATOR",
      actorSession: "stale",
      recordedAtUtc: "2026-04-30T10:11:00Z",
    });
    assert.throws(
      () => publishTerminalCloseoutRecord({
        wpId: "WP-TEST-TERMINAL-RECORD-v2",
        record: weaker,
        recordPathAbs: file,
      }),
      /downgrade/,
    );

    const reread = readTerminalCloseoutRecord({
      wpId: "WP-TEST-TERMINAL-RECORD-v2",
      recordPathAbs: file,
    });
    assert.equal(reread.status, "PRESENT");
    assert.equal(reread.record.terminal_state, "TERMINAL_SETTLED");
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("terminal closeout publication rejects same-rank stale writers", () => {
  const current = buildTerminalCloseoutRecordFromCloseoutSync({
    wpId: "WP-TEST-TERMINAL-RECORD-v3",
    mode: "FAIL",
    packetStatus: "Validated (FAIL)",
    taskBoardStatus: "DONE_FAIL",
    mainContainmentStatus: "NOT_REQUIRED",
    verdict: "FAIL",
    governanceDebtKeys: ["ACTIVE_TOPOLOGY_ARTIFACT_HYGIENE"],
    actorRole: "INTEGRATION_VALIDATOR",
    actorSession: "intv:new",
    recordedAtUtc: "2026-04-30T10:20:00Z",
  });
  const stale = buildTerminalCloseoutRecordFromCloseoutSync({
    wpId: "WP-TEST-TERMINAL-RECORD-v3",
    mode: "FAIL",
    packetStatus: "Validated (FAIL)",
    taskBoardStatus: "DONE_FAIL",
    mainContainmentStatus: "NOT_REQUIRED",
    verdict: "FAIL",
    governanceDebtKeys: ["ACTIVE_TOPOLOGY_ARTIFACT_HYGIENE"],
    actorRole: "ORCHESTRATOR",
    actorSession: "orc:old",
    recordedAtUtc: "2026-04-30T10:19:59Z",
  });

  const decision = resolveTerminalCloseoutPublication({
    currentRecord: current,
    nextRecord: stale,
  });
  assert.equal(decision.ok, false);
  assert.equal(decision.code, "TERMINAL_STALE_WRITER_REJECTED");
});
