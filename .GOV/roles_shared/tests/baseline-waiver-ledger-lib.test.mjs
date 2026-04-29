import assert from "node:assert/strict";
import test from "node:test";

import {
  activeWaiversForPath,
  evaluateWaiverCoverage,
  normalizeBaselineCompileWaiver,
} from "../scripts/lib/baseline-waiver-ledger-lib.mjs";

const activeWaiver = normalizeBaselineCompileWaiver({
  waiver_id: "BCW-TEST-001",
  wp_id: "WP-TEST",
  status: "ACTIVE",
  blocker_command: "cargo test --workspace",
  allowed_edit_paths: ["src/backend/handshake_core/build.rs", "src/backend/handshake_core/src/**"],
  operator_authority_ref: "operator approved baseline compile repair",
});

test("activeWaiversForPath matches exact paths and directory globs", () => {
  assert.equal(activeWaiversForPath("src/backend/handshake_core/build.rs", [activeWaiver]).length, 1);
  assert.equal(activeWaiversForPath("src/backend/handshake_core/src/lib.rs", [activeWaiver]).length, 1);
  assert.equal(activeWaiversForPath("src/frontend/app.tsx", [activeWaiver]).length, 0);
});

test("evaluateWaiverCoverage reports uncovered paths", () => {
  const coverage = evaluateWaiverCoverage({
    paths: ["src/backend/handshake_core/build.rs", "src/frontend/app.tsx"],
    waivers: [activeWaiver],
  });

  assert.equal(coverage.ok, false);
  assert.deepEqual(coverage.covered.map((entry) => entry.path), ["src/backend/handshake_core/build.rs"]);
  assert.deepEqual(coverage.uncovered.map((entry) => entry.path), ["src/frontend/app.tsx"]);
});

test("evaluateWaiverCoverage ignores closed or final-outcome waivers", () => {
  const closed = normalizeBaselineCompileWaiver({
    ...activeWaiver,
    waiver_id: "BCW-TEST-002",
    status: "CLOSED",
  });
  const expired = normalizeBaselineCompileWaiver({
    ...activeWaiver,
    waiver_id: "BCW-TEST-003",
    final_outcome: "proof command passed",
  });

  assert.equal(activeWaiversForPath("src/backend/handshake_core/build.rs", [closed, expired]).length, 0);
});
