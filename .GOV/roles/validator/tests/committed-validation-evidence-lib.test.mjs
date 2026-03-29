import assert from "node:assert/strict";
import test from "node:test";
import {
  committedEvidenceForCloseout,
  committedEvidenceHasDurablePass,
  livePrepareWorktreeHealthEvidence,
  normalizeCommittedValidationEvidence,
  recordCommittedValidationRun,
} from "../scripts/lib/committed-validation-evidence-lib.mjs";

function runFixture({
  status = "PASS",
  targetHeadSha = "abc123",
  validatedAt = "2026-03-29T10:00:00Z",
} = {}) {
  return {
    wp_id: "WP-TEST-COMMITTED-EVIDENCE-v1",
    status,
    validated_at: validatedAt,
    prepare_worktree_dir: "../wtc-test",
    committed_validation_mode: "HEAD",
    committed_validation_target: "HEAD",
    target_head_sha: targetHeadSha,
    pre_work_status: status,
    cargo_clean_status: status,
    post_work_status: status,
  };
}

test("recordCommittedValidationRun preserves the last successful committed target proof across later live failures", () => {
  const passing = recordCommittedValidationRun(null, runFixture({
    status: "PASS",
    targetHeadSha: "aaa111",
    validatedAt: "2026-03-29T10:00:00Z",
  }));
  const mixed = recordCommittedValidationRun(passing, runFixture({
    status: "FAIL",
    targetHeadSha: "bbb222",
    validatedAt: "2026-03-29T11:00:00Z",
  }));

  const durableProof = committedEvidenceForCloseout(mixed);
  const liveHealth = livePrepareWorktreeHealthEvidence(mixed);

  assert.equal(durableProof.status, "PASS");
  assert.equal(durableProof.target_head_sha, "aaa111");
  assert.equal(liveHealth.status, "FAIL");
  assert.equal(liveHealth.target_head_sha, "bbb222");
  assert.equal(committedEvidenceHasDurablePass(mixed), true);
});

test("normalizeCommittedValidationEvidence upgrades legacy flat evidence into the v2 structure", () => {
  const normalized = normalizeCommittedValidationEvidence({
    wp_id: "WP-TEST-COMMITTED-EVIDENCE-v1",
    status: "PASS",
    validated_at: "2026-03-29T10:00:00Z",
    prepare_worktree_dir: "../wtc-test",
    committed_validation_target: "HEAD",
    target_head_sha: "aaa111",
  });

  assert.equal(normalized.schema_version, "committed_validation_evidence_v2");
  assert.equal(normalized.latest_run.status, "PASS");
  assert.equal(normalized.last_successful_committed_target_proof.target_head_sha, "aaa111");
  assert.equal(normalized.proof_history.length >= 0, true);
});
