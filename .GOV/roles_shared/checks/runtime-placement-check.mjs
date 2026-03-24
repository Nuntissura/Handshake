import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  LEGACY_SHARED_GOV_GIT_TOPOLOGY_FILE,
  LEGACY_SHARED_GOV_RUNTIME_ROOT,
  LEGACY_SHARED_GOV_SESSION_CONTROL_OUTPUT_DIR,
  LEGACY_SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE,
  LEGACY_ORCHESTRATOR_GATES_FILE,
  LEGACY_SHARED_GOV_VALIDATOR_GATES_ROOT,
  LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT,
  SHARED_GOV_VALIDATOR_GATES_ROOT,
  SHARED_GOV_WP_COMMUNICATIONS_ROOT,
} from "../scripts/lib/runtime-paths.mjs";

const violations = [];
const ALLOWED_REPO_LOCAL_RUNTIME_ROOT_ENTRIES = new Set([
  "PRODUCT_GOVERNANCE_SNAPSHOT.json",
  "validator_gates",
]);

function gitStatusEntries(relPath) {
  try {
    const output = execFileSync(
      "git",
      ["status", "--porcelain=1", "--untracked-files=all", "--", relPath],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    ).trim();
    if (!output) return [];
    return output.split(/\r?\n/).filter(Boolean);
  } catch {
    return [];
  }
}

if (fs.existsSync(LEGACY_ORCHESTRATOR_GATES_FILE)) {
  violations.push(
    `${LEGACY_ORCHESTRATOR_GATES_FILE}: repo-local ORCHESTRATOR_GATES runtime file detected; live gate state must live under ../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json`,
  );
}

if (fs.existsSync(LEGACY_SHARED_GOV_RUNTIME_ROOT)) {
  const rootEntries = fs.readdirSync(LEGACY_SHARED_GOV_RUNTIME_ROOT, { withFileTypes: true });
  for (const entry of rootEntries) {
    if (ALLOWED_REPO_LOCAL_RUNTIME_ROOT_ENTRIES.has(entry.name)) continue;
    const relPath = path.posix.join(LEGACY_SHARED_GOV_RUNTIME_ROOT, entry.name);
    violations.push(
      `${relPath}: disallowed repo-local runtime residue detected; only PRODUCT_GOVERNANCE_SNAPSHOT.json and archive-only validator_gates/ may remain under ${LEGACY_SHARED_GOV_RUNTIME_ROOT}`,
    );
  }
}

if (fs.existsSync(LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT)) {
  const entries = fs.readdirSync(LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT, { withFileTypes: true });
  for (const entry of entries) {
    violations.push(
      `${path.posix.join(LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT, entry.name)}: repo-local WP communication runtime residue detected; live artifacts must live under ${SHARED_GOV_WP_COMMUNICATIONS_ROOT}`,
    );
  }
}

for (const entry of gitStatusEntries(LEGACY_SHARED_GOV_VALIDATOR_GATES_ROOT)) {
  violations.push(
    `${entry}: repo-local validator gate runtime residue detected under ${LEGACY_SHARED_GOV_VALIDATOR_GATES_ROOT}; live validator gate state must live under ${SHARED_GOV_VALIDATOR_GATES_ROOT} and the repo-local validator_gates tree is archive-only`,
  );
}

for (const relPath of [
  LEGACY_SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE,
  LEGACY_SHARED_GOV_SESSION_CONTROL_OUTPUT_DIR,
  LEGACY_SHARED_GOV_GIT_TOPOLOGY_FILE,
]) {
  if (!fs.existsSync(relPath)) continue;
  violations.push(
    `${relPath}: repo-local session/control/topology runtime residue detected; canonical location is external gov_runtime and this file/directory must not remain under ${LEGACY_SHARED_GOV_RUNTIME_ROOT}`,
  );
}

if (violations.length > 0) {
  console.error("runtime-placement-check: FAIL - repo-local governance runtime leakage detected");
  for (const violation of violations) console.error(`  - ${violation}`);
  process.exit(1);
}

console.log("runtime-placement-check ok");
