import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  buildWpCommunicationTemplateReplacements,
  findUnreplacedTemplateTokens,
} from "../scripts/wp/ensure-wp-communications.mjs";

const repoRoot = path.resolve(".");

function templateTokens(text) {
  return [...new Set(String(text || "").match(/\{\{[A-Z0-9_]+\}\}/g) || [])].sort();
}

test("WP communication template replacements cover all current template tokens", () => {
  const replacements = buildWpCommunicationTemplateReplacements({
    wpId: "WP-TEST-COMMS-v1",
    baseWpId: "WP-TEST-COMMS",
    dateIso: "2026-03-31T20:00:00.000Z",
    workflowLane: "ORCHESTRATOR_MANAGED",
    executionOwner: "CODER_A",
    workflowAuthority: "ORCHESTRATOR",
    technicalAdvisor: "WP_VALIDATOR",
    technicalAuthority: "INTEGRATION_VALIDATOR",
    mergeAuthority: "INTEGRATION_VALIDATOR",
    wpValidatorOfRecord: "null",
    integrationValidatorOfRecord: "null",
    secondaryValidatorSessionsJson: "[]",
    localBranch: "feat/WP-TEST-COMMS-v1",
    localWorktreeDir: "../wtc-test-comms-v1",
    agenticMode: "NO",
    packetStatus: "Ready for Dev",
    mainContainmentStatusJson: "\"NOT_STARTED\"",
    mergedMainCommitJson: "null",
    mainContainmentVerifiedAtUtcJson: "null",
    currentMainCompatibilityStatusJson: "\"NOT_RUN\"",
    currentMainCompatibilityBaselineShaJson: "null",
    currentMainCompatibilityVerifiedAtUtcJson: "null",
    packetWideningDecisionJson: "\"NONE\"",
    packetWideningEvidenceJson: "null",
    taskPacketPath: ".GOV/task_packets/WP-TEST-COMMS-v1/packet.md",
    communicationDir: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1",
    threadFile: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/THREAD.md",
    runtimeStatusFile: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/RUNTIME_STATUS.json",
    receiptsFile: "../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-TEST-COMMS-v1/RECEIPTS.jsonl",
    heartbeatIntervalMinutes: "15",
    heartbeatDueAt: "2026-03-31T20:15:00.000Z",
    staleAfter: "2026-03-31T20:45:00.000Z",
    maxCoderRevisionCycles: "3",
    maxValidatorReviewCycles: "3",
    maxRelayEscalationCycles: "2",
  });

  const templates = [
    ".GOV/templates/WP_COMMUNICATION_THREAD_TEMPLATE.md",
    ".GOV/templates/WP_RUNTIME_STATUS_TEMPLATE.json",
    ".GOV/templates/WP_RECEIPTS_TEMPLATE.jsonl",
  ].map((relPath) => fs.readFileSync(path.resolve(repoRoot, relPath), "utf8"));

  const uncovered = new Set();
  for (const token of templates.flatMap((text) => templateTokens(text))) {
    if (!(token in replacements)) uncovered.add(token);
  }

  assert.deepEqual([...uncovered], []);
});

test("findUnreplacedTemplateTokens returns sorted unique placeholders", () => {
  assert.deepEqual(
    findUnreplacedTemplateTokens("alpha {{SECOND}} beta {{FIRST}} {{SECOND}}"),
    ["{{FIRST}}", "{{SECOND}}"],
  );
  assert.deepEqual(findUnreplacedTemplateTokens("plain text only"), []);
});
