import assert from "node:assert/strict";
import test from "node:test";

import {
  VIOLATION_CLASSES,
  buildHbrViolation,
  canonicalJsonLine,
  validateHbrViolation,
} from "./hbr-violation-emit.mjs";

const EXPECTED_CANONICAL = "{\"emitted_at_utc\":\"2026-05-18T00:00:00Z\",\"evaluation_point\":\"build\",\"evidence_pointer\":\"test://hbr_violation_wire_contract\",\"hbr_id\":\"HBR-INT-001\",\"mt_id\":\"MT-006\",\"notes\":\"wire contract fixture\",\"receipt_kind\":\"HBR_VIOLATION\",\"receipt_uuid\":\"018f6d3a-1f00-7a2b-8c3d-123456789abc\",\"role\":\"KERNEL_BUILDER\",\"schema_version\":1,\"source_session\":\"KERNEL_BUILDER-20260518-012310\",\"violation_class\":\"MISSING_EVIDENCE\",\"wp_id\":\"WP-KERNEL-004-TEST\"}\n";

function fixtureViolation(overrides = {}) {
  return buildHbrViolation({
    receipt_uuid: "018f6d3a-1f00-7a2b-8c3d-123456789abc",
    hbr_id: "HBR-INT-001",
    wp_id: "WP-KERNEL-004-TEST",
    mt_id: "MT-006",
    role: "KERNEL_BUILDER",
    evaluation_point: "build",
    evidence_pointer: "test://hbr_violation_wire_contract",
    violation_class: "MISSING_EVIDENCE",
    emitted_at_utc: "2026-05-18T00:00:00Z",
    source_session: "KERNEL_BUILDER-20260518-012310",
    notes: "wire contract fixture",
    ...overrides,
  });
}

test("Node emitter outputs canonical JSONL matching the Rust fixture", () => {
  const violation = fixtureViolation();

  assert.deepEqual(validateHbrViolation(violation), []);
  assert.equal(canonicalJsonLine(violation), EXPECTED_CANONICAL);
});

test("all violation class variants validate", () => {
  for (const violationClass of VIOLATION_CLASSES) {
    const errors = validateHbrViolation(fixtureViolation({ violation_class: violationClass }));
    assert.deepEqual(errors, []);
  }
});
