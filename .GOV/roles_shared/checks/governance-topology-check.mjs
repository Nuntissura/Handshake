#!/usr/bin/env node

import {
  compareGovernanceTopologyProjection,
  GOVERNANCE_TOPOLOGY_REPO_REL_PATH,
} from "../scripts/lib/governance-topology-lib.mjs";

const result = compareGovernanceTopologyProjection();
if (!result.ok) {
  for (const error of result.errors) {
    console.error(`FAIL: ${error}`);
  }
  console.error(`EXPECTED_SURFACES: ${result.expected?.public_surface_summary?.total_surfaces ?? "unknown"}`);
  console.error(`LEDGER: ${GOVERNANCE_TOPOLOGY_REPO_REL_PATH}`);
  process.exit(1);
}

console.log("governance-topology-check ok");
