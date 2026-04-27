import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  appendCheckDetails,
  checkDetailsLogPath,
  compactCheckSummary,
  createCheckResult,
  formatCheckResultSummary,
  readCheckDetails,
  recordCheckResult,
  runSubprocessCheckStep,
} from "../scripts/lib/check-result-lib.mjs";

test("createCheckResult validates verdict, summary, and structured details", () => {
  const result = createCheckResult({
    verdict: "ok",
    summary: "task-board-check ok",
    details: { rows_checked: 250 },
  });

  assert.equal(result.schema_id, "hsk.check_result@1");
  assert.equal(result.verdict, "OK");
  assert.equal(result.summary, "task-board-check ok");
  assert.deepEqual(result.details, { rows_checked: 250 });
  assert.equal(formatCheckResultSummary(result), "OK | task-board-check ok");

  assert.throws(() => createCheckResult({ verdict: "PASS", summary: "x", details: {} }), /verdict/i);
  assert.throws(() => createCheckResult({ verdict: "OK", summary: "x".repeat(121), details: {} }), /summary/i);
  assert.throws(() => createCheckResult({ verdict: "OK", summary: "x", details: [] }), /details/i);
});

test("compactCheckSummary trims long model-visible summaries", () => {
  const summary = compactCheckSummary("x".repeat(140));
  assert.equal(summary.length, 120);
  assert.match(summary, /\.\.\.$/);
});

test("check detail log uses repo-scope and WP-scope paths", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "check-result-paths-"));
  try {
    assert.equal(checkDetailsLogPath({ runtimeRootAbs: root }), path.join(root, "check_details.jsonl"));
    assert.equal(
      checkDetailsLogPath({ runtimeRootAbs: root, wpId: "WP-TEST-v1" }),
      path.join(root, "roles_shared", "WP_COMMUNICATIONS", "WP-TEST-v1", "check_details.jsonl"),
    );
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("recordCheckResult appends details and returns the model-visible summary line", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "check-result-record-"));
  try {
    const recorded = recordCheckResult({
      check: "unit-check",
      verdict: "OK",
      summary: "unit-check ok",
      details: { line_count: 2 },
      timestamp: "2026-04-26T20:05:00.000Z",
      runtimeRootAbs: root,
    });

    assert.equal(recorded.summaryLine, "OK | unit-check ok");
    assert.equal(recorded.writeResult.appended, true);
    assert.equal(readCheckDetails({ runtimeRootAbs: root })[0].details.line_count, 2);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("runSubprocessCheckStep captures stdout and stderr into structured details", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "check-result-subprocess-"));
  const scriptPath = path.join(root, "sample-check.mjs");
  fs.writeFileSync(scriptPath, [
    "console.log('sample stdout');",
    "console.error('sample stderr');",
  ].join("\n"), "utf8");

  try {
    const step = runSubprocessCheckStep({
      check: "sample-check",
      scriptPath,
      cwd: root,
      runtimeRootAbs: root,
    });

    assert.equal(step.ok, true);
    assert.equal(step.summaryLine, "OK | sample-check ok");
    const rows = readCheckDetails({ runtimeRootAbs: root });
    assert.equal(rows.length, 1);
    assert.equal(rows[0].details.stdout.trim(), "sample stdout");
    assert.equal(rows[0].details.stderr.trim(), "sample stderr");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("appendCheckDetails is append-only and idempotent for identical entries", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "check-result-log-"));
  try {
    const result = createCheckResult({
      verdict: "FAIL",
      summary: "phase-check blocked",
      details: { blockers: ["MISSING_VERDICT"], output: ["line 1", "line 2"] },
    });
    const first = appendCheckDetails({
      check: "phase-check",
      wpId: "WP-TEST-v1",
      phase: "CLOSEOUT",
      result,
      timestamp: "2026-04-26T20:00:00.000Z",
      runtimeRootAbs: root,
    });
    const second = appendCheckDetails({
      check: "phase-check",
      wpId: "WP-TEST-v1",
      phase: "CLOSEOUT",
      result,
      timestamp: "2026-04-26T20:01:00.000Z",
      runtimeRootAbs: root,
    });
    const third = appendCheckDetails({
      check: "phase-check",
      wpId: "WP-TEST-v1",
      phase: "CLOSEOUT",
      result: createCheckResult({
        verdict: "WARN",
        summary: "projection debt only",
        details: { debt_keys: ["DOSSIER_SYNC"] },
      }),
      runtimeRootAbs: root,
    });

    assert.equal(first.appended, true);
    assert.equal(second.appended, false);
    assert.equal(third.appended, true);
    const rows = readCheckDetails({ wpId: "WP-TEST-v1", runtimeRootAbs: root });
    assert.equal(rows.length, 2);
    assert.equal(rows[0].summary, "phase-check blocked");
    assert.equal(rows[1].summary, "projection debt only");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
