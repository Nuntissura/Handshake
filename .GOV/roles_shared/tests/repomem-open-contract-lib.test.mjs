import assert from "node:assert/strict";
import test from "node:test";

import { validateRepomemOpenContract } from "../scripts/memory/repomem-open-contract-lib.mjs";

test("governed validator repomem open requires explicit role and wp binding", () => {
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
      providedRole: "WP_VALIDATOR",
      roleFlagProvided: true,
      wpId: "not-a-wp",
    }),
    /valid --wp WP-\{ID\}/i,
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

test("repomem open contract preserves non-governed defaults and accepts bound validator opens", () => {
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
      wpId: "",
    }),
    {
      role: "VALIDATOR",
      wpId: "",
    },
  );
});
