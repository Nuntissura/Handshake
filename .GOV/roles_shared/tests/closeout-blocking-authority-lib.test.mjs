import assert from "node:assert/strict";
import test from "node:test";

import {
  authorityClassForCloseoutDependency,
  dependencyBlocksProductOutcome,
  resolveArtifactHygieneCloseoutPolicy,
} from "../scripts/lib/closeout-blocking-authority-lib.mjs";

test("closeout dependency authority class marks correctness-critical dependencies explicitly", () => {
  assert.equal(authorityClassForCloseoutDependency("candidate_target"), "PRODUCT_CORRECTNESS");
  assert.equal(authorityClassForCloseoutDependency("scope_compatibility"), "PRODUCT_CORRECTNESS");
  assert.equal(authorityClassForCloseoutDependency("topology"), "GOVERNANCE_SUPPORT");
});

test("dependencyBlocksProductOutcome only trips for required FAIL dependencies in the product-correctness class", () => {
  assert.equal(
    dependencyBlocksProductOutcome({
      key: "candidate_target",
      required: true,
      status: "FAIL",
    }),
    true,
  );
  assert.equal(
    dependencyBlocksProductOutcome({
      key: "topology",
      required: true,
      status: "FAIL",
    }),
    false,
  );
  assert.equal(
    dependencyBlocksProductOutcome({
      key: "scope_compatibility",
      required: false,
      status: "FAIL",
    }),
    false,
  );
});

test("artifact hygiene is demoted to settlement debt for terminal non-pass closeout modes only", () => {
  const failPolicy = resolveArtifactHygieneCloseoutPolicy({ closeoutMode: "FAIL" });
  assert.equal(failPolicy.disposition, "SETTLEMENT_DEBT");
  assert.equal(failPolicy.debt_key, "ACTIVE_TOPOLOGY_ARTIFACT_HYGIENE");

  const mergePendingPolicy = resolveArtifactHygieneCloseoutPolicy({ closeoutMode: "MERGE_PENDING" });
  assert.equal(mergePendingPolicy.disposition, "BLOCK");
  assert.equal(mergePendingPolicy.debt_key, "");
});
