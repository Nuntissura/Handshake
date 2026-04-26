import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  appendCheckDetails,
  checkDetailsLogPath,
  createCheckResult,
  readCheckDetails,
} from "../scripts/lib/check-result-lib.mjs";

test("check details log appends repo and WP scoped rows without rewriting duplicates", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "check-details-log-"));
  try {
    const repoResult = createCheckResult({
      verdict: "OK",
      summary: "gov-check ok",
      details: { total_checks: 16 },
    });
    const wpResult = createCheckResult({
      verdict: "FAIL",
      summary: "phase-check blocked",
      details: { blocker: "validator-packet-complete" },
    });

    const repoWrite = appendCheckDetails({
      check: "gov-check",
      result: repoResult,
      timestamp: "2026-04-26T21:00:00.000Z",
      runtimeRootAbs: root,
    });
    const wpWrite = appendCheckDetails({
      check: "phase-check",
      wpId: "WP-TEST-v1",
      phase: "CLOSEOUT",
      result: wpResult,
      timestamp: "2026-04-26T21:01:00.000Z",
      runtimeRootAbs: root,
    });
    const duplicateWpWrite = appendCheckDetails({
      check: "phase-check",
      wpId: "WP-TEST-v1",
      phase: "CLOSEOUT",
      result: wpResult,
      timestamp: "2026-04-26T21:02:00.000Z",
      runtimeRootAbs: root,
    });

    assert.equal(repoWrite.appended, true);
    assert.equal(wpWrite.appended, true);
    assert.equal(duplicateWpWrite.appended, false);
    assert.equal(checkDetailsLogPath({ runtimeRootAbs: root }), path.join(root, "check_details.jsonl"));
    assert.equal(readCheckDetails({ runtimeRootAbs: root }).length, 1);
    assert.equal(readCheckDetails({ wpId: "WP-TEST-v1", runtimeRootAbs: root }).length, 1);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
