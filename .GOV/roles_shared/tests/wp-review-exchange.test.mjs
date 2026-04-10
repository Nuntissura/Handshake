import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { deriveFallbackReviewMicrotaskContract } from "../scripts/wp/wp-review-exchange.mjs";

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
