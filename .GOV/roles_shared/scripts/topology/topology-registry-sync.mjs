#!/usr/bin/env node

import {
  TOPOLOGY_REGISTRY_JSON_PATH,
  TOPOLOGY_REGISTRY_MD_PATH,
  absFromRepo,
  buildTopologyRegistry,
  renderTopologyRegistryMd,
  writeFileNormalized,
} from "./git-topology-lib.mjs";

const registry = buildTopologyRegistry();
writeFileNormalized(absFromRepo(TOPOLOGY_REGISTRY_JSON_PATH), `${JSON.stringify(registry, null, 2)}\n`);
writeFileNormalized(absFromRepo(TOPOLOGY_REGISTRY_MD_PATH), renderTopologyRegistryMd(registry));
console.log(`topology-registry-sync ok: ${TOPOLOGY_REGISTRY_JSON_PATH}`);
