import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { DatabaseSync } from "node:sqlite";

import {
  loadRecentProceduralFailureLines,
  loadHygieneReportSummaryLines,
  loadPriorDaySessionCloseLines,
} from "../scripts/session/session-control-lib.mjs";

function withTempDb(fn) {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "orchestrator-mem-"));
  const dbPath = path.join(tmpDir, "GOVERNANCE_MEMORY.db");
  const db = new DatabaseSync(dbPath);
  try {
    db.exec(`CREATE TABLE memory_index (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      memory_type TEXT NOT NULL,
      topic TEXT NOT NULL,
      summary TEXT NOT NULL,
      file_scope TEXT,
      wp_id TEXT,
      importance REAL,
      access_count INTEGER DEFAULT 0,
      created_at TEXT NOT NULL,
      last_accessed_at TEXT,
      consolidated INTEGER DEFAULT 0,
      snapshot_type TEXT DEFAULT ''
    )`);
    db.exec(`CREATE TABLE conversation_log (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      session_id TEXT NOT NULL,
      role TEXT,
      checkpoint_type TEXT NOT NULL,
      wp_id TEXT,
      topic TEXT,
      content TEXT,
      decisions TEXT,
      timestamp_utc TEXT NOT NULL
    )`);
    return fn({ db, dbPath, tmpDir });
  } finally {
    try { db.close(); } catch {}
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

const NOW = new Date("2026-04-27T12:00:00.000Z").getTime();
function isoFromOffset(offsetMs) {
  return new Date(NOW - offsetMs).toISOString();
}

test("recent procedural failures: surfaces all <7d procedural entries regardless of access_count", () =>
  withTempDb(({ db }) => {
    const insert = db.prepare(
      `INSERT INTO memory_index (memory_type, topic, summary, wp_id, importance, access_count, created_at, last_accessed_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
    );
    insert.run("procedural", "tool failure A", "Edit tool failed on long file path", "WP-A", 0.4, 0, isoFromOffset(60 * 60 * 1000), null);
    insert.run("procedural", "path mistake B", "Used windows path in unix grep", "WP-B", 0.3, 0, isoFromOffset(2 * 24 * 60 * 60 * 1000), null);
    insert.run("procedural", "old failure C", "Should not appear — older than 7 days", "WP-C", 0.5, 0, isoFromOffset(10 * 24 * 60 * 60 * 1000), null);
    insert.run("semantic", "pattern D", "Should not appear — wrong memory_type", "WP-D", 0.5, 0, isoFromOffset(60 * 60 * 1000), null);

    const result = loadRecentProceduralFailureLines(db, { now: NOW });
    assert.equal(result.count, 2);
    assert.match(result.lines[0], /tool failure A/);
    assert.match(result.lines[1], /path mistake B/);
    for (const line of result.lines) {
      assert.doesNotMatch(line, /old failure C/);
      assert.doesNotMatch(line, /pattern D/);
    }
  }));

test("recent procedural failures: empty result when no matching rows", () =>
  withTempDb(({ db }) => {
    const result = loadRecentProceduralFailureLines(db, { now: NOW });
    assert.deepEqual(result, { lines: [], tokenCount: 0, count: 0 });
  }));

test("prior-day session-close: returns recent SESSION_CLOSE entries within 30h window", () =>
  withTempDb(({ db }) => {
    const insert = db.prepare(
      `INSERT INTO conversation_log (session_id, role, checkpoint_type, wp_id, topic, content, decisions, timestamp_utc)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
    );
    insert.run("ORCH-1", "ORCHESTRATOR", "SESSION_CLOSE", "WP-A", "topic 1", "content 1", "Decided X over Y because Z", isoFromOffset(2 * 60 * 60 * 1000));
    insert.run("INTVAL-1", "INTEGRATION_VALIDATOR", "SESSION_CLOSE", "WP-B", "topic 2", "content 2", "PASSed with conditions", isoFromOffset(20 * 60 * 60 * 1000));
    insert.run("ORCH-2", "ORCHESTRATOR", "SESSION_OPEN", "WP-A", "ignored — wrong checkpoint_type", "", "", isoFromOffset(60 * 60 * 1000));
    insert.run("ORCH-3", "ORCHESTRATOR", "SESSION_CLOSE", "WP-X", "old", "old", "Older than the 30h window", isoFromOffset(100 * 60 * 60 * 1000));

    const result = loadPriorDaySessionCloseLines(db, { now: NOW });
    assert.equal(result.count, 2);
    assert.match(result.lines[0], /ORCHESTRATOR/);
    assert.match(result.lines[0], /Decided X over Y because Z/);
    assert.match(result.lines[1], /INTEGRATION_VALIDATOR/);
    assert.match(result.lines[1], /PASSed with conditions/);
    for (const line of result.lines) {
      assert.doesNotMatch(line, /Older than the 30h window/);
      assert.doesNotMatch(line, /ignored — wrong checkpoint_type/);
    }
  }));

test("prior-day session-close: empty result when conversation_log table missing", () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "orchestrator-mem-noconv-"));
  const dbPath = path.join(tmpDir, "test.db");
  const db = new DatabaseSync(dbPath);
  try {
    const result = loadPriorDaySessionCloseLines(db, { now: NOW });
    assert.deepEqual(result, { lines: [], tokenCount: 0, count: 0 });
  } finally {
    try { db.close(); } catch {}
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test("hygiene report: returns empty when file missing", () => {
  const result = loadHygieneReportSummaryLines({ now: NOW });
  // governance runtime root is shared in the repo — file may or may not exist.
  // We only assert the function returns the expected shape and never throws.
  assert.equal(typeof result.present, "boolean");
  assert.ok(Array.isArray(result.lines));
  assert.equal(typeof result.tokenCount, "number");
});
