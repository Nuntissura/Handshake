#!/usr/bin/env node

import fs from "node:fs";
import {
  TOPOLOGY_REGISTRY_JSON_PATH,
  TOPOLOGY_REGISTRY_MD_PATH,
  absFromRepo,
  buildTopologyRegistry,
  renderTopologyRegistryMd,
} from "../git-topology-lib.mjs";

function normalizeEol(value) {
  return String(value || "").replace(/\r\n/g, "\n");
}

const expectedJson = `${JSON.stringify(buildTopologyRegistry(), null, 2)}\n`;
const expectedMd = renderTopologyRegistryMd(buildTopologyRegistry());
const jsonPath = absFromRepo(TOPOLOGY_REGISTRY_JSON_PATH);
const mdPath = absFromRepo(TOPOLOGY_REGISTRY_MD_PATH);

const errors = [];
if (!fs.existsSync(jsonPath)) {
  errors.push(`Missing ${TOPOLOGY_REGISTRY_JSON_PATH}`);
} else if (normalizeEol(fs.readFileSync(jsonPath, "utf8")) !== normalizeEol(expectedJson)) {
  errors.push(`${TOPOLOGY_REGISTRY_JSON_PATH} is stale; run just topology-registry-sync`);
}

if (!fs.existsSync(mdPath)) {
  errors.push(`Missing ${TOPOLOGY_REGISTRY_MD_PATH}`);
} else if (normalizeEol(fs.readFileSync(mdPath, "utf8")) !== normalizeEol(expectedMd)) {
  errors.push(`${TOPOLOGY_REGISTRY_MD_PATH} is stale; run just topology-registry-sync`);
}

if (errors.length > 0) {
  for (const error of errors) console.error(`FAIL: ${error}`);
  process.exit(1);
}

console.log("topology-registry-check ok");
