import fs from "node:fs";
import { validateMergeProgressionTruth } from "../scripts/lib/merge-progression-truth-lib.mjs";
import { listOfficialWorkPacketPaths, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("merge-progression-truth-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("merge-progression-truth-check.mjs", message, { role: "SHARED", details });
}

const violations = [];

for (const packetPath of listOfficialWorkPacketPaths()) {
  const packetText = fs.readFileSync(repoPathAbs(packetPath), "utf8");
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
