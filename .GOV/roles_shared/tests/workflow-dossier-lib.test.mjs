import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  appendWorkflowDossierEntry,
  formatRepomemDossierEntry,
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
