import fs from "node:fs";
import path from "node:path";
import {
  LEGACY_ORCHESTRATOR_GATES_FILE,
  LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT,
  SHARED_GOV_WP_COMMUNICATIONS_ROOT,
} from "../scripts/lib/runtime-paths.mjs";

const violations = [];

if (fs.existsSync(LEGACY_ORCHESTRATOR_GATES_FILE)) {
  violations.push(
    `${LEGACY_ORCHESTRATOR_GATES_FILE}: repo-local ORCHESTRATOR_GATES runtime file detected; live gate state must live under ../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json`,
  );
}

if (fs.existsSync(LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT)) {
  const entries = fs.readdirSync(LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT, { withFileTypes: true });
  for (const entry of entries) {
    violations.push(
      `${path.posix.join(LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT, entry.name)}: repo-local WP communication runtime residue detected; live artifacts must live under ${SHARED_GOV_WP_COMMUNICATIONS_ROOT}`,
    );
  }
}

if (violations.length > 0) {
  console.error("runtime-placement-check: FAIL - repo-local governance runtime leakage detected");
  for (const violation of violations) console.error(`  - ${violation}`);
  process.exit(1);
}

console.log("runtime-placement-check ok");
