import assert from "node:assert/strict";
import test from "node:test";

import {
  activeWaiversForPath,
  evaluateWaiverCoverage,
  normalizeBaselineCompileWaiver,
} from "../scripts/lib/baseline-waiver-ledger-lib.mjs";
import {
  POLICY_WAIVER_SIGNATURE_RE,
  loadRegisteredSignatures,
  parsePolicyWaiverLedger,
} from "../scripts/lib/computed-policy-gate-lib.mjs";

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

// [VPX-006] signed policy waivers: the WAIVERS GRANTED ledger is only ACTIVE when the SIGNATURE field
// is well-formed and registered in SIGNATURE_AUDIT.md. These tests use the real audit file via
// loadRegisteredSignatures() so "not in audit" is proven against the canonical registry.
test("policy waiver signed with a registered SIGNATURE_AUDIT signature is ACTIVE", () => {
  const registered = loadRegisteredSignatures();
  const [knownSignature] = [...registered];
  assert.ok(knownSignature, "SIGNATURE_AUDIT.md must contain at least one registered signature");
  const ledger = parsePolicyWaiverLedger([
    "## WAIVERS GRANTED",
    `- WAIVER_ID: CX-SIGNED-1 | STATUS: ACTIVE | COVERS: TEST | APPROVER: USER | SIGNATURE: ${knownSignature}`,
  ].join("\n"));
  assert.equal(ledger.entries[0].status, "ACTIVE");
  assert.equal(ledger.entries[0].signatureValid, true);
  assert.deepEqual(ledger.activeCoverageTokens, ["TEST"]);
});

test("policy waiver without SIGNATURE is UNSIGNED and does not count", () => {
  const ledger = parsePolicyWaiverLedger([
    "## WAIVERS GRANTED",
    "- WAIVER_ID: CX-UNSIGNED-1 | STATUS: ACTIVE | COVERS: TEST | APPROVER: USER",
  ].join("\n"));
  assert.equal(ledger.entries[0].status, "UNSIGNED");
  assert.equal(ledger.entries[0].signatureValid, false);
  assert.equal(ledger.activeEntries.length, 0);
  assert.deepEqual(ledger.activeCoverageTokens, []);
});

test("policy waiver with malformed signature is UNSIGNED and does not count", () => {
  const ledger = parsePolicyWaiverLedger([
    "## WAIVERS GRANTED",
    "- WAIVER_ID: CX-MALFORMED-1 | STATUS: ACTIVE | COVERS: TEST | APPROVER: USER | SIGNATURE: ILJA-01-01-2026",
  ].join("\n"));
  assert.equal(ledger.entries[0].status, "UNSIGNED");
  assert.equal(ledger.entries[0].signature, "ILJA-01-01-2026");
  assert.equal(ledger.entries[0].signatureValid, false);
  assert.equal(ledger.activeEntries.length, 0);
});

test("policy waiver with well-formed signature absent from SIGNATURE_AUDIT is UNSIGNED", () => {
  const fabricated = "zzznotregistered010120260101";
  assert.ok(POLICY_WAIVER_SIGNATURE_RE.test(fabricated));
  assert.equal(loadRegisteredSignatures().has(fabricated), false);
  const ledger = parsePolicyWaiverLedger([
    "## WAIVERS GRANTED",
    `- WAIVER_ID: CX-UNREGISTERED-1 | STATUS: ACTIVE | COVERS: TEST | APPROVER: USER | USER_SIGNATURE: ${fabricated}`,
  ].join("\n"));
  assert.equal(ledger.entries[0].status, "UNSIGNED");
  assert.equal(ledger.entries[0].signatureValid, false);
  assert.equal(ledger.activeEntries.length, 0);
});
