#!/usr/bin/env node

import { readGovernanceTopology, GOVERNANCE_TOPOLOGY_REPO_REL_PATH } from "../scripts/lib/governance-topology-lib.mjs";
import { buildTopologyRegistry } from "../scripts/topology/git-topology-lib.mjs";

const topology = readGovernanceTopology();
const expected = buildTopologyRegistry();
const actual = topology?.git_topology_contract?.protected_checkout_topology;

const errors = [];
if (!topology) {
  errors.push(`Missing ${GOVERNANCE_TOPOLOGY_REPO_REL_PATH}`);
} else if (!actual) {
  errors.push(`${GOVERNANCE_TOPOLOGY_REPO_REL_PATH} is missing git_topology_contract.protected_checkout_topology; run just gov-check --sync-topology`);
} else if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  errors.push(`${GOVERNANCE_TOPOLOGY_REPO_REL_PATH} git_topology_contract is stale; run just gov-check --sync-topology`);
}

if (errors.length > 0) {
  for (const error of errors) console.error(`FAIL: ${error}`);
  process.exit(1);
}

console.log("topology-registry-check ok (legacy registry folded into GOVERNANCE_TOPOLOGY.json)");
