import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  classifyHeuristicRiskText,
  isHeuristicRiskContract,
  mergeHeuristicRiskContract,
} from "../scripts/lib/heuristic-risk-lib.mjs";
import { listDeclaredWpMicrotasks } from "../scripts/lib/wp-microtask-lib.mjs";
import { deriveFallbackReviewMicrotaskContract } from "../scripts/wp/wp-review-exchange.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

function writePacketWithMicrotask({ wpId, mtText }) {
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  fs.writeFileSync(path.join(packetDir, "MT-001.md"), mtText, "utf8");
  return packetDir;
}

test("classifyHeuristicRiskText detects redaction and base64 discriminator risk", () => {
  const result = classifyHeuristicRiskText([
    "- CLAUSE: separate high-entropy base64 secrets from plausible identifiers",
    "- RISK_IF_MISSED: false positive and false negative counterexamples keep shifting thresholds",
  ].join("\n"));

  assert.equal(result.heuristic_risk, "YES");
  assert.equal(result.heuristic_risk_class, "SECRET_OR_IDENTIFIER_BOUNDARY");
  assert.equal(result.strategy_escalation, "DISCRIMINATOR_REDESIGN");
  assert.equal(result.repair_cycle_strategy_threshold, 2);
  assert.ok(result.required_evidence.includes("CORPUS_CASES"));
  assert.ok(result.required_evidence.includes("FALSE_POSITIVE_FALSE_NEGATIVE_PROBES"));
});

test("classifyHeuristicRiskText leaves deterministic storage tasks untagged", () => {
  const result = classifyHeuristicRiskText([
    "- CLAUSE: add a required database column and migration",
    "- RISK_IF_MISSED: writes fail at startup",
  ].join("\n"));

  assert.equal(result.heuristic_risk, "NO");
  assert.equal(result.heuristic_risk_class, "NONE");
  assert.deepEqual(result.required_evidence, []);
});

test("mergeHeuristicRiskContract adds bounded strategy contract fields", () => {
  const classification = classifyHeuristicRiskText("- CLAUSE: fuzzy classifier threshold with adversarial counterexamples");
  const merged = mergeHeuristicRiskContract({ scope_ref: "MT-001" }, classification);

  assert.equal(merged.scope_ref, "MT-001");
  assert.equal(merged.heuristic_risk, "YES");
  assert.equal(merged.repair_cycle_strategy_threshold, 2);
  assert.ok(isHeuristicRiskContract(merged));
});

test("microtask parsing and review fallback project heuristic-risk contract", () => {
  const wpId = "WP-TEST-HEURISTIC-RISK-v1";
  const packetDir = writePacketWithMicrotask({
    wpId,
    mtText: [
      "# MT-001: Redaction discriminator",
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- MT_ID: MT-001",
      "- CLAUSE: Redaction classifier separates base64 secrets from identifier prose",
      "- CODE_SURFACES: src/redaction.rs",
      "- EXPECTED_TESTS: cargo test redaction",
      "- DEPENDS_ON: NONE",
      "- RISK_IF_MISSED: false positives and false negatives keep passing review",
    ].join("\n"),
  });

  try {
    const microtasks = listDeclaredWpMicrotasks(wpId);
    assert.equal(microtasks.length, 1);
    assert.equal(microtasks[0].heuristicRisk.heuristic_risk, "YES");

    const contract = deriveFallbackReviewMicrotaskContract({
      wpId,
      receiptKind: "REVIEW_REQUEST",
      actorRole: "CODER",
      targetRole: "WP_VALIDATOR",
      packetRowRef: "MT-001",
      summary: "MT-001 complete: redaction discriminator",
    });

    assert.equal(contract.heuristic_risk, "YES");
    assert.equal(contract.expected_receipt_kind, "REVIEW_RESPONSE");
    assert.ok(contract.required_evidence.includes("NEGATIVE_COUNTEREXAMPLES"));
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});
