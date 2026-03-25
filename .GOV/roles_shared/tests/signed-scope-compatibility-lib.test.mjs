import assert from "node:assert/strict";
import test from "node:test";
import {
  packetRequiresSignedScopeCompatibility,
  validateSignedScopeCompatibilityTruth,
} from "../scripts/lib/signed-scope-compatibility-lib.mjs";

function packetWithCompatibility(fields) {
  return [
    "- PACKET_FORMAT_VERSION: 2026-03-26",
    `- CURRENT_MAIN_COMPATIBILITY_STATUS: ${fields.status}`,
    `- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: ${fields.baseline}`,
    `- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: ${fields.verifiedAt}`,
    `- PACKET_WIDENING_DECISION: ${fields.widening}`,
    `- PACKET_WIDENING_EVIDENCE: ${fields.evidence}`,
  ].join("\n");
}

test("packetRequiresSignedScopeCompatibility gates on 2026-03-26+", () => {
  assert.equal(packetRequiresSignedScopeCompatibility("2026-03-25"), false);
  assert.equal(packetRequiresSignedScopeCompatibility("2026-03-26"), true);
});

test("validateSignedScopeCompatibilityTruth accepts compatible current-main truth", () => {
  const sha = "0123456789abcdef0123456789abcdef01234567";
  const validation = validateSignedScopeCompatibilityTruth(
    packetWithCompatibility({
      status: "COMPATIBLE",
      baseline: sha,
      verifiedAt: "2026-03-26T10:00:00Z",
      widening: "NOT_REQUIRED",
      evidence: "N/A",
    }),
    {
      packetPath: "packet.md",
      currentMainHeadSha: sha,
      requireReadyForPass: true,
    },
  );

  assert.deepEqual(validation.errors, []);
});

test("validateSignedScopeCompatibilityTruth rejects stale current-main baseline", () => {
  const validation = validateSignedScopeCompatibilityTruth(
    packetWithCompatibility({
      status: "COMPATIBLE",
      baseline: "0123456789abcdef0123456789abcdef01234567",
      verifiedAt: "2026-03-26T10:00:00Z",
      widening: "NOT_REQUIRED",
      evidence: "N/A",
    }),
    {
      packetPath: "packet.md",
      currentMainHeadSha: "89abcdef0123456789abcdef0123456789abcdef",
      requireReadyForPass: true,
    },
  );

  assert.match(validation.errors.join("\n"), /does not match current local main HEAD/i);
});

test("validateSignedScopeCompatibilityTruth rejects adjacent scope requirement without governed widening decision", () => {
  const validation = validateSignedScopeCompatibilityTruth(
    packetWithCompatibility({
      status: "ADJACENT_SCOPE_REQUIRED",
      baseline: "0123456789abcdef0123456789abcdef01234567",
      verifiedAt: "2026-03-26T10:00:00Z",
      widening: "NONE",
      evidence: "N/A",
    }),
    {
      packetPath: "packet.md",
      requireReadyForPass: true,
    },
  );

  assert.match(validation.errors.join("\n"), /requires PACKET_WIDENING_DECISION=FOLLOW_ON_WP_REQUIRED\|SUPERSEDING_PACKET_REQUIRED/i);
  assert.match(validation.errors.join("\n"), /PASS-ready closeout prohibited/i);
});
