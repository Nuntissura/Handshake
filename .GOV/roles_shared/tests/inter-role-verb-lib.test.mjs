import assert from "node:assert/strict";
import test from "node:test";
import {
  interRoleVerbNames,
  loadInterRoleVerbSchema,
  renderInterRoleVerbReceipt,
  validateInterRoleVerbBody,
} from "../scripts/lib/inter-role-verb-lib.mjs";

const VALID_BODIES = {
  MT_HANDOFF: { mt_id: "MT-001", range: "abc..def", commit: "def", summary: "Implemented MT-001." },
  MT_VERDICT: { mt_id: "MT-001", verdict: "PASS", concerns: [], track: "JUDGMENT" },
  MT_REMEDIATION_REQUIRED: { mt_id: "MT-001", concerns: ["missing negative test"], next_action: "Add regression test." },
  WP_HANDOFF: { wp_id: "WP-TEST-VERB-v1", final_range: "abc..def", mts_completed: ["MT-001"], summary: "WP ready." },
  INTEGRATION_VERDICT: { wp_id: "WP-TEST-VERB-v1", verdict: "PASS", mechanical_track: "PASS", judgment_track: "PASS", closeout_path: ".GOV/reports/closeout.md" },
  CONCERN: { concern_class: "SCOPE_RISK", severity: "HIGH", evidence_path: ".GOV/reports/concern.md", notes: "Risk needs Orchestrator review." },
  PHASE_TRANSITION: { wp_id: "WP-TEST-VERB-v1", from_phase: "IMPLEMENTATION", to_phase: "VALIDATION", provenance: "phase-check HANDOFF" },
  RELAUNCH_REQUEST: { wp_id: "WP-TEST-VERB-v1", target_role: "CODER", reason: "Repair required", priority: "urgent" },
};

test("inter-role verb registry exposes schemas and validates each initial verb", () => {
  assert.deepEqual(interRoleVerbNames().sort(), Object.keys(VALID_BODIES).sort());
  for (const [verb, body] of Object.entries(VALID_BODIES)) {
    assert.ok(loadInterRoleVerbSchema(verb), `${verb} schema should load`);
    const result = validateInterRoleVerbBody(verb, body);
    assert.equal(result.ok, true, `${verb}: ${result.errors.join("; ")}`);
  }
});

test("inter-role verb validation fails closed for missing required fields and ad-hoc verbs", () => {
  assert.equal(validateInterRoleVerbBody("AD_HOC_VERB", {}).ok, false);
  const result = validateInterRoleVerbBody("MT_VERDICT", { mt_id: "MT-001", verdict: "PASS", track: "JUDGMENT" });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("; "), /MT_VERDICT\.concerns is required/);
});

test("inter-role verb renderer produces human-readable projection text", () => {
  const line = renderInterRoleVerbReceipt({
    verb: "MT_VERDICT",
    verb_body: { mt_id: "MT-001", verdict: "FAIL", concerns: ["missing proof"], track: "JUDGMENT" },
  });
  assert.match(line, /MT_VERDICT MT-001: FAIL\/JUDGMENT/);
  assert.match(line, /missing proof/);
});

test("MT_VERDICT accepts typed mechanical concerns", () => {
  const result = validateInterRoleVerbBody("MT_VERDICT", {
    mt_id: "MT-001",
    verdict: "FAIL",
    concerns: [{ key: "OUTSIDE_MT_CONTRACT", severity: "HIGH", evidence_path: ".GOV/task_packets/WP/MT-001.md" }],
    track: "MECHANICAL",
  });
  assert.equal(result.ok, true, result.errors.join("; "));

  const line = renderInterRoleVerbReceipt({
    verb: "MT_VERDICT",
    verb_body: result.body,
  });
  assert.match(line, /OUTSIDE_MT_CONTRACT:HIGH/);
});
