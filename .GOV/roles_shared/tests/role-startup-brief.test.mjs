import assert from "node:assert/strict";
import test from "node:test";
import {
  ROLE_STARTUP_BRIEF_PATHS,
  formatStartupBriefForRole,
  loadStartupBriefsForRole,
  validateStartupBriefShape,
} from "../scripts/memory/role-startup-brief-lib.mjs";

test("startup brief files exist and validate for every governed role", () => {
  for (const role of Object.keys(ROLE_STARTUP_BRIEF_PATHS)) {
    const brief = loadStartupBriefsForRole(role);
    assert.deepEqual(brief.errors, [], `${role}: ${brief.errors.join("; ")}`);
    assert.match(brief.roleContent, new RegExp(`ROLE: ${role}`));
  }
});

test("startup brief formatter includes shared and role-specific operational memory", () => {
  const output = formatStartupBriefForRole("ORCHESTRATOR");
  assert.match(output, /STARTUP_BRIEF_BEGIN role=ORCHESTRATOR/);
  assert.match(output, /# Shared Startup Brief/);
  assert.match(output, /# Orchestrator Startup Brief/);
  assert.match(output, /RAM-ORCHESTRATOR-SESSION_OPEN-001/);
  assert.match(output, /STARTUP_BRIEF_END role=ORCHESTRATOR/);
});

test("startup brief shape rejects missing action card fields", () => {
  const errors = validateStartupBriefShape({
    role: "CODER",
    content: [
      "# CODER Startup Brief",
      "## Status",
      "- SCHEMA_VERSION: `hsk.startup_brief@1`",
      "## Use",
      "## Action Cards",
      "### RAM-CODER-TEST-001",
      "- ACTION: TEST",
    ].join("\n"),
  });
  assert.ok(errors.some((error) => error.includes("DO_NOT")));
  assert.ok(errors.some((error) => error.includes("VERIFY")));
});
