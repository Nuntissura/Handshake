import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  deriveFallbackReviewMicrotaskContract,
  parseReviewExchangeCliArgs,
  resolveReviewAckFor,
} from "../scripts/wp/wp-review-exchange.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

function writeMicrotask(packetDir, wpId, mtId, clause, codeSurfaces, expectedTests) {
  fs.writeFileSync(
    path.join(packetDir, `${mtId}.md`),
    [
      `# ${mtId}: ${clause}`,
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      `- MT_ID: ${mtId}`,
      `- CLAUSE: ${clause}`,
      `- CODE_SURFACES: ${codeSurfaces.join("; ")}`,
      `- EXPECTED_TESTS: ${expectedTests.join("; ")}`,
      "- DEPENDS_ON: NONE",
    ].join("\n"),
    "utf8",
  );
}

test("deriveFallbackReviewMicrotaskContract synthesizes overlap metadata from MT packet_row_ref", () => {
  const wpId = "WP-TEST-REVIEW-EXCHANGE-FALLBACK-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(
    packetDir,
    wpId,
    "MT-003",
    "Boundary scope [CX-MT-401]",
    ["runtime_governance.rs", "role_mailbox.rs"],
    ["cargo test governance_workflow_mirror_boundary -- --exact"],
  );

  try {
    const contract = deriveFallbackReviewMicrotaskContract({
      wpId,
      receiptKind: "REVIEW_REQUEST",
      actorRole: "CODER",
      targetRole: "WP_VALIDATOR",
      packetRowRef: "MT-003",
      summary: "MT-003 review_mode=OVERLAP: boundary proof is ready for review.",
      microtaskContract: null,
    });

    assert.deepEqual(contract, {
      scope_ref: "MT-003",
      file_targets: ["runtime_governance.rs", "role_mailbox.rs"],
      proof_commands: ["cargo test governance_workflow_mirror_boundary -- --exact"],
      phase_gate: "MICROTASK",
      review_mode: "OVERLAP",
      expected_receipt_kind: "REVIEW_RESPONSE",
    });
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("parseReviewExchangeCliArgs maps named validator kickoff metadata without positional shift", () => {
  const parsed = parseReviewExchangeCliArgs([
    "VALIDATOR_KICKOFF",
    "WP-TEST-v1",
    "WP_VALIDATOR",
    "wpv:test",
    "CODER",
    "coder:test",
    "Kickoff",
    "spec_anchor=Spec v1",
    "packet_row_ref=AC-001",
    "microtask_json={\"scope_ref\":\"MT-001\"}",
  ]);

  assert.equal(parsed.correlationId, undefined);
  assert.equal(parsed.specAnchor, "Spec v1");
  assert.equal(parsed.packetRowRef, "AC-001");
  assert.equal(parsed.microtaskJson, "{\"scope_ref\":\"MT-001\"}");
});

test("parseReviewExchangeCliArgs unwraps Just wrapper key=value metadata", () => {
  const parsed = parseReviewExchangeCliArgs([
    "CODER_INTENT",
    "WP-TEST-v1",
    "CODER",
    "coder:test",
    "WP_VALIDATOR",
    "wpv:test",
    "Intent",
    "correlation_id=correlation_id=kickoff-1",
    "spec_anchor=spec_anchor=Spec v1",
    "packet_row_ref=packet_row_ref=MT-001",
    "ack_for=ack_for=kickoff-1",
    "microtask_json=microtask_json={\"scope_ref\":\"MT-001\"}",
  ]);

  assert.equal(parsed.correlationId, "kickoff-1");
  assert.equal(parsed.specAnchor, "Spec v1");
  assert.equal(parsed.packetRowRef, "MT-001");
  assert.equal(parsed.ackFor, "kickoff-1");
  assert.equal(parsed.microtaskJson, "{\"scope_ref\":\"MT-001\"}");
});

test("parseReviewExchangeCliArgs strips shell wrapper quotes from named metadata", () => {
  const parsed = parseReviewExchangeCliArgs([
    "CODER_HANDOFF",
    "WP-TEST-v1",
    "CODER",
    "coder:test",
    "WP_VALIDATOR",
    "wpv:test",
    "Handoff",
    "'microtask_json={\"scope_ref\":\"MT-001\",\"commit\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}'",
  ]);

  assert.equal(parsed.microtaskJson, "{\"scope_ref\":\"MT-001\",\"commit\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}");
});

test("deriveFallbackReviewMicrotaskContract infers coder handoff commit from summary", () => {
  const wpId = "WP-TEST-REVIEW-EXCHANGE-HANDOFF-COMMIT-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(
    packetDir,
    wpId,
    "MT-001",
    "Handoff range proof",
    ["runtime_governance.rs"],
    ["cargo test handoff_range -- --exact"],
  );

  try {
    const contract = deriveFallbackReviewMicrotaskContract({
      wpId,
      receiptKind: "CODER_HANDOFF",
      actorRole: "CODER",
      targetRole: "WP_VALIDATOR",
      packetRowRef: "MT-001",
      summary: "MT-001 CODER_HANDOFF: commit aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa pushed.",
      microtaskContract: null,
    });

    assert.equal(contract.commit, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert.equal(contract.scope_ref, "MT-001");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("resolveReviewAckFor defaults coder intent ack to correlation id", () => {
  assert.equal(resolveReviewAckFor({
    receiptKind: "CODER_INTENT",
    actorRole: "CODER",
    ackFor: "",
    correlationId: "kickoff-1",
  }), "kickoff-1");

  assert.equal(resolveReviewAckFor({
    receiptKind: "CODER_INTENT",
    actorRole: "CODER",
    ackFor: "explicit-ack",
    correlationId: "kickoff-1",
  }), "explicit-ack");
});
