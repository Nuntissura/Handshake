#!/usr/bin/env node

import {
  buildGovernanceTopology,
  writeGovernanceTopology,
  GOVERNANCE_TOPOLOGY_REPO_REL_PATH,
} from "../lib/governance-topology-lib.mjs";

writeGovernanceTopology(buildGovernanceTopology());
console.log(`topology-registry-sync deprecated: folded topology registry into ${GOVERNANCE_TOPOLOGY_REPO_REL_PATH}`);
console.log("Use just gov-check --sync-topology for the canonical topology refresh path.");
