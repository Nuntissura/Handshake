import assert from "node:assert/strict";
import test from "node:test";

import { buildContractDerivedPacketProjectionText } from "../scripts/lib/work-packet-contract-read-lib.mjs";
import { parseCurrentWpStatus } from "../scripts/lib/role-resume-utils.mjs";

test("JSON packet projection exposes mt_plan wp_status as Current WP_STATUS", () => {
  const projection = buildContractDerivedPacketProjectionText({
    contract: {
      wp_id: "WP-TEST-JSON-STATUS-v1",
      lifecycle: {
        status: "In Progress",
        workflow_lane: "KERNEL_BUILDER_FOLDED_NO_ACP",
        execution_owner: "KERNEL_BUILDER",
      },
      workflow: {
        lane: "KERNEL_BUILDER_FOLDED",
        execution_owner: "KERNEL_BUILDER",
        coder_compatible_execution_lane: "MULTI_AGENT_MT_PARALLEL",
      },
      mt_plan: {
        next_id: "WP_VALIDATION",
        wp_status: "IN_PROGRESS_WP_VALIDATION_AFTER_MT101_REWORK",
      },
    },
    source: "TEST",
  });

  assert.equal(
    parseCurrentWpStatus(projection),
    "IN_PROGRESS_WP_VALIDATION_AFTER_MT101_REWORK",
  );
  assert.match(projection, /^- WORKFLOW_LANE: KERNEL_BUILDER_FOLDED_NO_ACP$/m);
  assert.match(projection, /^- EXECUTION_OWNER: KERNEL_BUILDER$/m);
  assert.match(projection, /^- AGENTIC_MODE: YES$/m);
});
