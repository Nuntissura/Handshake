import fs from "node:fs";
import { validateMergeProgressionTruth } from "../scripts/lib/merge-progression-truth-lib.mjs";
import { listOfficialWorkPacketPaths } from "../scripts/lib/runtime-paths.mjs";

function fail(message, details = []) {
  console.error(`[MERGE_PROGRESSION_TRUTH_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

const violations = [];

for (const packetPath of listOfficialWorkPacketPaths()) {
  const packetText = fs.readFileSync(packetPath, "utf8");
  const result = validateMergeProgressionTruth(packetText, {
    packetPath,
  });
  for (const error of result.errors) {
    violations.push(`${packetPath.replace(/\\/g, "/")}: ${error}`);
  }
}

if (violations.length > 0) {
  fail("Merge progression truth drift detected", violations);
}

console.log("merge-progression-truth-check ok");
