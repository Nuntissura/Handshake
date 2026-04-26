import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  appendWorkflowDossierEntry,
  formatRepomemDossierEntry,
  formatRepomemDossierSnapshotEntry,
  selectRepomemEntriesForWorkflowDossier,
} from "../scripts/audit/workflow-dossier-lib.mjs";

function writeText(filePath, text) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, text, "utf8");
}

test("appendWorkflowDossierEntry skips consecutive duplicate sync payloads when a dedupe suffix is provided", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-workflow-dossier-lib-"));
  const dossierPath = path.join(tempRoot, ".GOV", "Audits", "smoketest", "DOSSIER_TEST.md");

  try {
    writeText(
      dossierPath,
      [
        "# Dossier",
        "",
        "## LIVE_EXECUTION_LOG",
        "",
        "## LIVE_IDLE_LEDGER",
        "",
      ].join("\n"),
    );

    const executionPayload = "[ORCHESTRATOR] [ACP_SYNC] [MECHANICAL] `BROKER(0 active) -> WP-TEST [working / waiting_on=CODER_HANDOFF]` | sessions=1 | control=1/1";
    appendWorkflowDossierEntry({
      repoRoot: tempRoot,
      filePath: dossierPath,
      section: "EXECUTION",
      line: `- [2026-04-22 09:00:00 Europe/Brussels] ${executionPayload}`,
      dedupeSuffix: executionPayload,
    });
    appendWorkflowDossierEntry({
      repoRoot: tempRoot,
      filePath: dossierPath,
      section: "EXECUTION",
      line: `- [2026-04-22 09:05:00 Europe/Brussels] ${executionPayload}`,
      dedupeSuffix: executionPayload,
    });
    appendWorkflowDossierEntry({
      repoRoot: tempRoot,
      filePath: dossierPath,
      section: "EXECUTION",
      line: "- [2026-04-22 09:06:00 Europe/Brussels] [ORCHESTRATOR] [ACP_SYNC] [MECHANICAL] `BROKER(1 active) -> WP-TEST [working / waiting_on=CODER_HANDOFF]` | sessions=1 | control=2/2",
      dedupeSuffix: "[ORCHESTRATOR] [ACP_SYNC] [MECHANICAL] `BROKER(1 active) -> WP-TEST [working / waiting_on=CODER_HANDOFF]` | sessions=1 | control=2/2",
    });

    const content = fs.readFileSync(dossierPath, "utf8");
    const executionLines = content
      .split(/\r?\n/)
      .filter((line) => line.startsWith("- [2026-04-22"));
    assert.equal(executionLines.length, 2);
    assert.match(executionLines[0], /BROKER\(0 active\)/);
    assert.match(executionLines[1], /BROKER\(1 active\)/);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("appendWorkflowDossierEntry matches section headings with descriptive suffixes", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-workflow-dossier-lib-"));
  const dossierPath = path.join(tempRoot, ".GOV", "Audits", "smoketest", "DOSSIER_TEST.md");

  try {
    writeText(
      dossierPath,
      [
        "# Dossier",
        "",
        "## LIVE_EXECUTION_LOG (mechanical telemetry and closeout imports)",
        "",
        "Existing notes.",
        "",
      ].join("\n"),
    );

    appendWorkflowDossierEntry({
      repoRoot: tempRoot,
      filePath: dossierPath,
      section: "EXECUTION",
      line: "- [2026-04-25 10:00:00 Europe/Brussels] [ORCHESTRATOR] [NOTE] tolerant heading append",
    });

    const content = fs.readFileSync(dossierPath, "utf8");
    const headingCount = content.split(/\r?\n/)
      .filter((line) => line.startsWith("## LIVE_EXECUTION_LOG")).length;
    assert.equal(headingCount, 1);
    assert.match(content, /tolerant heading append/);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("appendWorkflowDossierEntry prepends orchestrator diagnostics near the top newest-first", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-workflow-dossier-lib-"));
  const dossierPath = path.join(tempRoot, ".GOV", "Audits", "smoketest", "DOSSIER_TEST.md");

  try {
    writeText(
      dossierPath,
      [
        "# Dossier",
        "",
        "## METADATA",
        "",
        "- ACTIVE_RECOVERY_PACKET: WP-TEST-DOSSIER-v1",
        "",
        "---",
        "",
        "## 1. Executive Summary",
        "",
        "- Existing summary.",
        "",
      ].join("\n"),
    );

    appendWorkflowDossierEntry({
      repoRoot: tempRoot,
      filePath: dossierPath,
      section: "ORCHESTRATOR_DIAGNOSTIC",
      line: "- [2026-04-25 10:00:00 Europe/Brussels] [ORCHESTRATOR] [FIRST] first live note",
      insertMode: "section-prepend",
    });
    appendWorkflowDossierEntry({
      repoRoot: tempRoot,
      filePath: dossierPath,
      section: "ORCHESTRATOR_DIAGNOSTIC",
      line: "- [2026-04-25 10:05:00 Europe/Brussels] [ORCHESTRATOR] [SECOND] second live note",
      insertMode: "section-prepend",
    });

    const content = fs.readFileSync(dossierPath, "utf8");
    assert(content.indexOf("## LIVE_ORCHESTRATOR_DIAGNOSTIC_LOG") < content.indexOf("## 1. Executive Summary"));
    assert(content.indexOf("[SECOND] second live note") < content.indexOf("[FIRST] first live note"));
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("appendWorkflowDossierEntry creates ACP trace at EOF and appends downward", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-workflow-dossier-lib-"));
  const dossierPath = path.join(tempRoot, ".GOV", "Audits", "smoketest", "DOSSIER_TEST.md");

  try {
    writeText(
      dossierPath,
      [
        "# Dossier",
        "",
        "## LIVE_FINDINGS_LOG",
        "",
        "- Existing finding.",
        "",
      ].join("\n"),
    );

    appendWorkflowDossierEntry({
      repoRoot: tempRoot,
      filePath: dossierPath,
      section: "ACP_TRACE",
      line: "- [2026-04-25 10:00:00 Europe/Brussels] [ORCHESTRATOR] [ACP_UPDATE] first acp event",
    });
    appendWorkflowDossierEntry({
      repoRoot: tempRoot,
      filePath: dossierPath,
      section: "ACP_TRACE",
      line: "- [2026-04-25 10:05:00 Europe/Brussels] [ORCHESTRATOR] [ACP_SESSION_CONTROL] second acp event",
    });

    const content = fs.readFileSync(dossierPath, "utf8");
    assert(content.trimEnd().endsWith("second acp event"));
    assert(content.indexOf("first acp event") < content.indexOf("second acp event"));
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("selectRepomemEntriesForWorkflowDossier imports WP-bound session context without cross-WP global bleed", () => {
  const selected = selectRepomemEntriesForWorkflowDossier([
    {
      session_id: "CODER-20260422-090000",
      role: "CODER",
      checkpoint_type: "SESSION_OPEN",
      wp_id: "",
      timestamp_utc: "2026-04-22T09:00:00.000Z",
      topic: "coder global open",
      content: "Global open belongs to the same session that later wrote a WP-bound checkpoint.",
    },
    {
      session_id: "CODER-20260422-090000",
      role: "CODER",
      checkpoint_type: "DECISION",
      wp_id: "WP-TEST-DOSSIER-v1",
      timestamp_utc: "2026-04-22T09:01:00.000Z",
      topic: "coder WP decision",
      content: "The coder recorded a concrete decision tied to the target WP.",
    },
    {
      session_id: "CODER-20260422-090000",
      role: "CODER",
      checkpoint_type: "SESSION_CLOSE",
      wp_id: "",
      timestamp_utc: "2026-04-22T09:02:00.000Z",
      topic: "coder global close",
      content: "Global close belongs to the same session that wrote the WP-bound decision.",
    },
    {
      session_id: "CODER-20260422-100000",
      role: "CODER",
      checkpoint_type: "SESSION_OPEN",
      wp_id: "",
      timestamp_utc: "2026-04-22T10:00:00.000Z",
      topic: "other coder global open",
      content: "This global session belongs to another parallel WP and must not be imported.",
    },
    {
      session_id: "CODER-20260422-100000",
      role: "CODER",
      checkpoint_type: "DECISION",
      wp_id: "WP-OTHER-v1",
      timestamp_utc: "2026-04-22T10:01:00.000Z",
      topic: "other coder decision",
      content: "This entry is tied to another WP and must not bleed into the target dossier.",
    },
  ], { wpId: "WP-TEST-DOSSIER-v1" });

  assert.deepEqual(
    selected.map((entry) => entry.topic),
    ["coder global open", "coder WP decision", "coder global close"],
  );
});

test("formatRepomemDossierEntry maps decisions to governance-memory execution lines", () => {
  const formatted = formatRepomemDossierEntry({
    session_id: "INTEGRATION_VALIDATOR-20260422-110000",
    role: "INTEGRATION_VALIDATOR",
    checkpoint_type: "DECISION",
    wp_id: "WP-TEST-DOSSIER-v1",
    timestamp_utc: "2026-04-22T11:00:00.000Z",
    topic: "final verdict recorded before merge",
    content: "The validator recorded the whole-WP verdict and the conditions that must hold before main containment.",
  });

  assert.equal(formatted.section, "EXECUTION");
  assert.equal(formatted.tag, "REPOMEM_DECISION");
  assert.match(formatted.line, /\[INTEGRATION_VALIDATOR\] \[REPOMEM_DECISION\] \[GOVERNANCE_MEMORY\]/);
  assert.match(formatted.line, /final verdict recorded before merge :: The validator recorded/);
});

test("formatRepomemDossierSnapshotEntry keeps full terminal memory payload", () => {
  const longContent = "The terminal memory snapshot should preserve the full role-authored diagnostic payload ".repeat(8).trim();
  const formatted = formatRepomemDossierSnapshotEntry({
    id: 42,
    session_id: "INTEGRATION_VALIDATOR-20260425-110000",
    role: "INTEGRATION_VALIDATOR",
    checkpoint_type: "DECISION",
    wp_id: "WP-TEST-DOSSIER-v1",
    timestamp_utc: "2026-04-25T11:00:00.000Z",
    topic: "final verdict recorded",
    content: longContent,
  });

  assert.equal(formatted.section, "TERMINAL_REPOMEM");
  assert.equal(formatted.tag, "REPOMEM_DECISION");
  assert.match(formatted.line, /\[INTEGRATION_VALIDATOR\] \[REPOMEM_DECISION\] \[GOVERNANCE_MEMORY\]/);
  assert.match(formatted.line, /wp=WP-TEST-DOSSIER-v1 id=42/);
  assert(formatted.line.includes(longContent));
});
