import assert from "node:assert/strict";
import test from "node:test";

import { validateRepomemOpenContract } from "../scripts/memory/repomem-open-contract-lib.mjs";

test("WP-bound role repomem open requires explicit role and wp binding", () => {
  assert.throws(
    () => validateRepomemOpenContract({
      providedRole: "INTEGRATION_VALIDATOR",
      roleFlagProvided: true,
      wpId: "",
    }),
    /requires --wp WP-\{ID\}/i,
  );
  assert.throws(
    () => validateRepomemOpenContract({
      providedRole: "CODER",
      roleFlagProvided: true,
      wpId: "not-a-wp",
    }),
    /valid --wp WP-\{ID\}/i,
  );
  assert.throws(
    () => validateRepomemOpenContract({
      providedRole: "ACTIVATION_MANAGER",
      roleFlagProvided: false,
      wpId: "WP-TEST-v1",
    }),
    /requires explicit --role ACTIVATION_MANAGER/i,
  );
  assert.throws(
    () => validateRepomemOpenContract({
      providedRole: "",
      roleFlagProvided: false,
      wpId: "WP-TEST-v1",
      environmentRole: "INTEGRATION_VALIDATOR",
    }),
    /requires explicit --role INTEGRATION_VALIDATOR/i,
  );
});

test("repomem open contract preserves non-WP-bound defaults and accepts bound role opens", () => {
  assert.deepEqual(
    validateRepomemOpenContract({
      providedRole: "",
      roleFlagProvided: false,
      wpId: "",
    }),
    {
      role: "ORCHESTRATOR",
      wpId: "",
    },
  );
  assert.deepEqual(
    validateRepomemOpenContract({
      providedRole: "INTEGRATION_VALIDATOR",
      roleFlagProvided: true,
      wpId: "WP-TEST-VALIDATOR-v1",
    }),
    {
      role: "INTEGRATION_VALIDATOR",
      wpId: "WP-TEST-VALIDATOR-v1",
    },
  );
  assert.deepEqual(
    validateRepomemOpenContract({
      providedRole: "VALIDATOR",
      roleFlagProvided: true,
      wpId: "WP-TEST-CLASSIC-VALIDATOR-v1",
    }),
    {
      role: "VALIDATOR",
      wpId: "WP-TEST-CLASSIC-VALIDATOR-v1",
    },
  );
  assert.deepEqual(
    validateRepomemOpenContract({
      providedRole: "MEMORY_MANAGER",
      roleFlagProvided: true,
      wpId: "",
    }),
    {
      role: "MEMORY_MANAGER",
      wpId: "",
    },
  );
});
