#!/usr/bin/env node
/**
 * Spec regression check: ensure SPEC_CURRENT points to an existing spec and required anchors are present.
 */
import { readFileSync } from "node:fs";
import path from "node:path";
import { GOV_ROOT_REPO_REL, repoPathAbs } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const specPointerPath = `${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md`;
const requiredAnchors = [
  "2.3.12", // storage portability pillars
  "2.3.11", // retention/GC
  "2.6.7",  // semantic catalog
  "2.9.3",  // mutation traceability / silent edit guard
  "4.6",    // tokenization
];

function fail(message) {
  console.error(`validator-spec-regression: FAIL - ${message}`);
  process.exit(1);
}

function main() {
  let specPointer;
  try {
    specPointer = readFileSync(repoPathAbs(specPointerPath), "utf8");
  } catch (error) {
    fail(`cannot read ${specPointerPath}: ${error.message}`);
  }

  const match = specPointer.match(/\*\*([^*\r\n]*Handshake_Master_Spec_[^*]+)\*\*/);
  if (!match) {
    fail("SPEC_CURRENT does not reference a Master Spec filename.");
  }

  const specRef = match[1].trim();
  const specFile = specRef.split("/").pop();
  const specPath = path.isAbsolute(specRef)
    ? specRef
    : specRef.startsWith(".GOV/")
      ? repoPathAbs(specRef)
      : repoPathAbs(path.join(GOV_ROOT_REPO_REL, "spec", specRef));

  let spec;
  try {
    spec = readFileSync(specPath, "utf8");
  } catch (error) {
    fail(`cannot read referenced spec ${specPath}: ${error.message}`);
  }

  for (const anchor of requiredAnchors) {
    if (!spec.includes(anchor)) {
      fail(`required spec anchor "${anchor}" missing in ${specFile}`);
    }
  }

  console.log(`validator-spec-regression: PASS - ${specFile} present with required anchors.`);
}

main();
