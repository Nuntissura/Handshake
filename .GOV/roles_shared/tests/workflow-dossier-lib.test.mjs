import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { appendWorkflowDossierEntry } from "../scripts/audit/workflow-dossier-lib.mjs";

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
