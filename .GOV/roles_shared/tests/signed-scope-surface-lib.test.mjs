import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  validateCandidateTargetAgainstSignedScope,
  validateContainedMainCommitAgainstSignedScope,
} from "../scripts/lib/signed-scope-surface-lib.mjs";

function writeFile(targetPath, content) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, content, "utf8");
}

function packetFixture() {
  return [
    "# Task Packet: WP-TEST-SIGNED-SCOPE-v1",
    "",
    "## METADATA",
    "- WP_ID: WP-TEST-SIGNED-SCOPE-v1",
    "- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main",
    "",
    "## VALIDATION",
    "- **Target File**: `src/demo.rs`",
    "- **Start**: 10",
    "- **End**: 20",
    "- **Line Delta**: 3",
    "- **Artifacts**: `artifacts/signed.patch`",
  ].join("\n");
}

const matchingDiff = [
  "diff --git a/src/demo.rs b/src/demo.rs",
  "--- a/src/demo.rs",
  "+++ b/src/demo.rs",
  "@@ -10 +10,2 @@",
  "-old",
  "+new",
  "+extra",
  "",
].join("\n");

test("validateCandidateTargetAgainstSignedScope passes when candidate diff matches the declared signed surface", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "signed-scope-surface-pass-"));
  writeFile(path.join(tempRoot, "artifacts", "signed.patch"), matchingDiff);

  const result = validateCandidateTargetAgainstSignedScope(packetFixture(), {
    repoRoot: tempRoot,
    targetHeadSha: "abc1234",
    currentMainHeadSha: "def5678",
    candidateDiffText: matchingDiff,
  });

  assert.equal(result.ok, true);
  assert.deepEqual(result.errors, []);
});

test("validateCandidateTargetAgainstSignedScope fails when the candidate diff widens beyond the declared file surface", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "signed-scope-surface-fail-"));
  writeFile(path.join(tempRoot, "artifacts", "signed.patch"), matchingDiff);
  const widenedDiff = [
    matchingDiff.trimEnd(),
    "diff --git a/src/other.rs b/src/other.rs",
    "--- a/src/other.rs",
    "+++ b/src/other.rs",
    "@@ -1 +1 @@",
    "-x",
    "+y",
    "",
  ].join("\n");

  const result = validateCandidateTargetAgainstSignedScope(packetFixture(), {
    repoRoot: tempRoot,
    targetHeadSha: "abc1234",
    currentMainHeadSha: "def5678",
    candidateDiffText: widenedDiff,
  });

  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /undeclared file changed/i);
});

test("validateContainedMainCommitAgainstSignedScope fails when merged main diff drifts from the signed patch artifact", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "signed-scope-contained-fail-"));
  writeFile(path.join(tempRoot, "artifacts", "signed.patch"), matchingDiff);
  const driftedDiff = [
    "diff --git a/src/demo.rs b/src/demo.rs",
    "--- a/src/demo.rs",
    "+++ b/src/demo.rs",
    "@@ -10 +10,3 @@",
    "-old",
    "+new",
    "+extra",
    "+drift",
    "",
  ].join("\n");

  const result = validateContainedMainCommitAgainstSignedScope(packetFixture(), {
    repoRoot: tempRoot,
    mergedMainCommit: "abc1234",
    actualDiffText: driftedDiff,
  });

  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /does not match the signed patch artifact/i);
});
