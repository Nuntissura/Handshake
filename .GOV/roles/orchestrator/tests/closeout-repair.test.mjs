import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  applySignedScopePatchRepair,
  collectCloseoutRepairFailures,
  resolveDeclaredPatchArtifactPath,
} from "../scripts/closeout-repair.mjs";

test("closeout-repair classifies failures from direct helper truth instead of phase-check output text", () => {
  const packetText = [
    "# Task Packet: WP-TEST-CLOSEOUT-v1",
    "",
    "**Status:** Done",
    "",
    "## METADATA",
    "- PACKET_FORMAT_VERSION: 2026-03-26",
    "- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE",
    "- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 1111111111111111111111111111111111111111",
    "- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-03-26T10:00:00Z",
    "- PACKET_WIDENING_DECISION: NOT_REQUIRED",
    "- PACKET_WIDENING_EVIDENCE: N/A",
    "- **Artifacts**: `artifacts/signed.patch`",
  ].join("\n");

  const result = collectCloseoutRepairFailures({
    packetText,
    packetPath: ".GOV/task_packets/WP-TEST-CLOSEOUT-v1/packet.md",
    currentMainHeadSha: "2222222222222222222222222222222222222222",
    validatorPacketCompleteResult: {
      ok: false,
      message: "validator-packet-complete: FAIL - packet completeness still has unresolved metadata",
    },
    communicationHealthResult: {
      ok: false,
      message: "Direct review route projection or notification boundary is inconsistent",
      details: ["pending_notification=THREAD_MESSAGE:CODER->WP_VALIDATOR:abc123:Please review"],
    },
    closeoutResult: {
      ok: false,
      message: "Integration-validator topology or closeout bundle is not ready",
      details: ["request cmd-1 has no settled result"],
    },
    signedScopeSurfaceValidation: {
      ok: false,
      errors: ["signed scope patch artifact is missing: D:/temp/artifacts/signed.patch"],
    },
    clauseConsistencyValidation: {
      errors: ["VALIDATION_REPORTS CLAUSES_REVIEWED missing clause from CLAUSE_CLOSURE_MATRIX: CX-123"],
    },
  });

  assert.deepEqual(
    result.failures.map((failure) => failure.code),
    [
      "BASELINE_SHA_MISMATCH",
      "MISSING_SIGNED_SCOPE_PATCH",
      "CLAUSE_COVERAGE_MISMATCH",
      "MISSING_VALIDATION_VERDICT",
      "PACKET_COMPLETENESS_OTHER",
      "COMMUNICATION_HEALTH",
      "INTEGRATION_VALIDATOR_CLOSEOUT",
    ],
  );
});

test("closeout-repair writes regenerated patch to the packet-declared artifact path", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-closeout-repair-"));
  const packetPath = ".GOV/task_packets/WP-TEST-CLOSEOUT-v1/packet.md";
  const packetText = [
    "# Task Packet: WP-TEST-CLOSEOUT-v1",
    "",
    "## METADATA",
    "- **Artifacts**: `artifacts/custom-signed.patch`",
    "- **MERGE_BASE_SHA**: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`",
    "- **COMMITTED_TARGET_HEAD_SHA**: `bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb`",
  ].join("\n");

  try {
    const declaredPath = resolveDeclaredPatchArtifactPath({
      packetText,
      packetPath,
      repoRoot: tempRoot,
    });
    const repair = applySignedScopePatchRepair({
      packetText,
      packetPath,
      repoRoot: tempRoot,
      gitExec: () => "diff --git a/src/demo.rs b/src/demo.rs\n+change\n",
    });

    assert.equal(repair.applied, true);
    assert.equal(repair.patchPath, "artifacts/custom-signed.patch");
    assert.equal(declaredPath, path.resolve(tempRoot, "artifacts/custom-signed.patch"));
    assert.equal(
      fs.readFileSync(path.resolve(tempRoot, repair.patchPath), "utf8"),
      "diff --git a/src/demo.rs b/src/demo.rs\n+change\n",
    );
    assert.equal(fs.existsSync(path.resolve(tempRoot, ".GOV/task_packets/WP-TEST-CLOSEOUT-v1/signed-scope.patch")), false);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});
