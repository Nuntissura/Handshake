import fs from "node:fs";
import { validateHistoricalSmoketestLineage } from "../scripts/lib/historical-smoketest-lineage-lib.mjs";
import { GOV_ROOT_REPO_REL, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";

const TRACE_REGISTRY_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`;
const TASK_BOARD_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`;

function fail(message, details = []) {
  console.error(`[HISTORICAL_SMOKETEST_LINEAGE_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

const registryText = fs.existsSync(repoPathAbs(TRACE_REGISTRY_PATH))
  ? fs.readFileSync(repoPathAbs(TRACE_REGISTRY_PATH), "utf8")
  : "";
const taskBoardText = fs.existsSync(repoPathAbs(TASK_BOARD_PATH))
  ? fs.readFileSync(repoPathAbs(TASK_BOARD_PATH), "utf8")
  : "";

const result = validateHistoricalSmoketestLineage({
  registryText,
  taskBoardText,
});

if (result.errors.length > 0) {
  fail("Historical-failure and live-smoketest lineage drift detected", result.errors);
}

console.log("historical-smoketest-lineage-check ok");
