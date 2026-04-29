import assert from "node:assert/strict";
import test from "node:test";

import { classifyFailureRecovery } from "../scripts/lib/failure-class-recovery-lib.mjs";

test("classifyFailureRecovery routes candidate target drift as product blocker", () => {
  const recovery = classifyFailureRecovery({
    wpId: "WP-TEST",
    dependencyView: {
      product_outcome_blocking_keys: ["candidate_target"],
      governance_debt_keys: [],
      blocking_keys: ["candidate_target"],
    },
    issues: ["candidate target diff does not match the signed patch artifact"],
  });

  assert.equal(recovery.failure_class, "PRODUCT_BLOCKER");
  assert.equal(recovery.revalidation_required, true);
  assert.equal(recovery.product_proof_preserved, false);
});

test("classifyFailureRecovery routes noncanonical artifact root as environment blocker", () => {
  const recovery = classifyFailureRecovery({
    wpId: "WP-TEST",
    issues: ["artifact hygiene detected a noncanonical sibling artifact root and cargo target cache"],
  });

  assert.equal(recovery.failure_class, "ENVIRONMENT_BLOCKER");
  assert.equal(recovery.revalidation_required, false);
  assert.equal(recovery.product_proof_preserved, true);
});

test("classifyFailureRecovery routes session and dossier debt as governance blocker", () => {
  const recovery = classifyFailureRecovery({
    wpId: "WP-TEST",
    dependencyView: {
      product_outcome_blocking_keys: [],
      governance_debt_keys: ["ORCHESTRATOR:NO_WP_DURABLE_CHECKPOINT"],
      blocking_keys: ["closeout_bundle"],
    },
    issues: ["Session CODER:WP-TEST still reports RUNNING", "dossier closeout judgment is incomplete"],
  });

  assert.equal(recovery.failure_class, "GOVERNANCE_BLOCKER");
  assert.equal(recovery.revalidation_required, false);
  assert.match(recovery.next_command, /closeout-repair/);
});
