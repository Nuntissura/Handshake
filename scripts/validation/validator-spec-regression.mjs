#!/usr/bin/env node
/**
 * Spec regression check: ensure SPEC_CURRENT points to existing spec and required anchors are present.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

const specPointerPath = "docs/SPEC_CURRENT.md";
// Phase/safety-critical anchors that must exist in the current spec.
const requiredAnchors = [
  "2.3.12", // storage portability pillars
  "2.3.11", // retention/GC
  "2.6.7",  // semantic catalog
  "2.9.3",  // mutation traceability / silent edit guard
  "4.6",    // tokenization
];

function fail(msg) {
  console.error(`validator-spec-regression: FAIL — ${msg}`);
  process.exit(1);
}

function main() {
  let specPointer;
  try {
    specPointer = readFileSync(specPointerPath, "utf8");
  } catch (err) {
    fail(`cannot read ${specPointerPath}: ${err.message}`);
  }

  const match = specPointer.match(/\*\*(Handshake_Master_Spec_[^*]+)\*\*/);
  if (!match) {
    fail("SPEC_CURRENT does not reference a Master Spec filename.");
  }
  const specFile = match[1];
  const specPath = join(specFile); // specs live at repo root

  let spec;
  try {
    spec = readFileSync(specPath, "utf8");
  } catch (err) {
    fail(`cannot read referenced spec ${specPath}: ${err.message}`);
  }

  for (const anchor of requiredAnchors) {
    if (!spec.includes(anchor)) {
      fail(`required spec anchor "${anchor}" missing in ${specFile}`);
    }
  }

  console.log(`validator-spec-regression: PASS — ${specFile} present with required anchors.`);
}

main();
