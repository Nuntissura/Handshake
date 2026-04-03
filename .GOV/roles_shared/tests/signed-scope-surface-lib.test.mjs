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

function packetFixture({ mergeBaseSha = "" } = {}) {
  const lines = [
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
  ];
  if (mergeBaseSha) lines.splice(4, 0, `- MERGE_BASE_SHA: ${mergeBaseSha}`);
  return lines.join("\n");
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

test("validateContainedMainCommitAgainstSignedScope can accept contained-main harmonization when exact artifact match is disabled", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "signed-scope-contained-relaxed-"));
  writeFile(path.join(tempRoot, "artifacts", "signed.patch"), matchingDiff);
  const harmonizedDiff = [
    "diff --git a/src/demo.rs b/src/demo.rs",
    "--- a/src/demo.rs",
    "+++ b/src/demo.rs",
    "@@ -8 +8 @@",
    "-legacy",
    "+legacy",
    "@@ -10 +10,2 @@",
    "-old",
    "+new",
    "+extra",
    "",
  ].join("\n");

  const result = validateContainedMainCommitAgainstSignedScope(packetFixture(), {
    repoRoot: tempRoot,
    mergedMainCommit: "abc1234",
    actualDiffText: harmonizedDiff,
    requireExactArtifactMatch: false,
  });

  assert.equal(result.ok, true);
  assert.deepEqual(result.errors, []);
});

test("validateCandidateTargetAgainstSignedScope tolerates current-main line shifts when the signed patch artifact still matches", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "signed-scope-surface-shift-"));
  const shiftedDiff = [
    "diff --git a/src/demo.rs b/src/demo.rs",
    "--- a/src/demo.rs",
    "+++ b/src/demo.rs",
    "@@ -5 +5,2 @@",
    "-old",
    "+new",
    "+extra",
    "",
  ].join("\n");
  writeFile(path.join(tempRoot, "artifacts", "signed.patch"), shiftedDiff);

  const result = validateCandidateTargetAgainstSignedScope(packetFixture(), {
    repoRoot: tempRoot,
    targetHeadSha: "abc1234",
    currentMainHeadSha: "def5678",
    candidateDiffText: shiftedDiff,
  });

  assert.equal(result.ok, true);
  assert.deepEqual(result.errors, []);
});

test("validateCandidateTargetAgainstSignedScope uses the target first-parent diff when current main already contains the target", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "signed-scope-surface-contained-"));
  writeFile(path.join(tempRoot, "artifacts", "signed.patch"), matchingDiff);

  const result = validateCandidateTargetAgainstSignedScope(packetFixture(), {
    repoRoot: tempRoot,
    targetHeadSha: "abc1234",
    currentMainHeadSha: "def5678",
    gitRunner: (args) => {
      if (args[0] === "merge-base" && args[1] === "--is-ancestor") return { code: 0, output: "" };
      if (args[0] === "merge-base") return { code: 0, output: "abc1234" };
      if (args[0] === "rev-list") return { code: 0, output: "abc1234 0123456" };
      if (args[0] === "diff" && args[3] === "0123456" && args[4] === "abc1234") {
        return { code: 0, output: matchingDiff };
      }
      return { code: 0, output: "" };
    },
  });

  assert.equal(result.ok, true);
  assert.deepEqual(result.errors, []);
});

test("validateCandidateTargetAgainstSignedScope honors MERGE_BASE_SHA for multi-commit signed ranges", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "signed-scope-surface-merge-base-"));
  writeFile(path.join(tempRoot, "artifacts", "signed.patch"), matchingDiff);
  const mergeBaseSha = "1111111111111111111111111111111111111111";
  const targetHeadSha = "2222222222222222222222222222222222222222";

  const result = validateCandidateTargetAgainstSignedScope(packetFixture({ mergeBaseSha }), {
    repoRoot: tempRoot,
    targetHeadSha,
    currentMainHeadSha: "3333333333333333333333333333333333333333",
    gitRunner: (args) => {
      if (args[0] === "merge-base" && args[1] === "--is-ancestor" && args[2] === mergeBaseSha && args[3] === targetHeadSha) {
        return { code: 0, output: "" };
      }
      if (args[0] === "diff" && args[3] === mergeBaseSha && args[4] === targetHeadSha) {
        return { code: 0, output: matchingDiff };
      }
      return { code: 0, output: "" };
    },
  });

  assert.equal(result.ok, true);
  assert.deepEqual(result.errors, []);
  assert.equal(result.mergeBaseSha, mergeBaseSha);
});
