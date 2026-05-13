#!/usr/bin/env node
/**
 * Spec regression check: ensure SPEC_CURRENT resolves and required anchors are present.
 */
import { GOV_ROOT_REPO_REL, REPO_ROOT } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  readResolvedSpecTextAtRepo,
  resolveSpecCurrentAtRepo,
} from "../../../roles_shared/scripts/lib/spec-current-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
registerFailCaptureHook("validator-spec-regression.mjs", { role: "WP_VALIDATOR" });

const specPointerPath = `${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md`;
const requiredAnchors = [
  "2.3.12", // storage portability pillars
  "2.3.11", // retention/GC
  "2.6.7",  // semantic catalog
  "2.9.3",  // mutation traceability / silent edit guard
  "4.6",    // tokenization
];

function fail(message) {
  failWithMemory("validator-spec-regression.mjs", message, { role: "WP_VALIDATOR" });
}

function main() {
  let resolved;
  let spec;
  try {
    resolved = resolveSpecCurrentAtRepo(REPO_ROOT, { allowLegacy: false });
    spec = readResolvedSpecTextAtRepo(REPO_ROOT, resolved);
  } catch (error) {
    fail(`cannot resolve current spec from ${specPointerPath}: ${error.message}`);
  }

  for (const anchor of requiredAnchors) {
    if (!spec.includes(anchor)) {
      fail(`required spec anchor "${anchor}" missing in ${resolved.specTargetLabel}`);
    }
  }

  console.log(`validator-spec-regression: PASS - ${resolved.specTargetLabel} resolves with required anchors.`);
}

main();
