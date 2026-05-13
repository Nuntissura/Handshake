import fs from "node:fs";
import { validateMergeProgressionTruth } from "../scripts/lib/merge-progression-truth-lib.mjs";
import { inferWpIdFromPacketPath, listOfficialWorkPacketPaths, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { readWorkPacketContract } from "../scripts/lib/work-packet-contract-read-lib.mjs";

registerFailCaptureHook("merge-progression-truth-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("merge-progression-truth-check.mjs", message, { role: "SHARED", details });
}

const violations = [];

for (const packetPath of listOfficialWorkPacketPaths()) {
  const wpId = inferWpIdFromPacketPath(packetPath);
  const contractState = wpId ? readWorkPacketContract(wpId) : null;
  const contract = contractState?.contract && typeof contractState.contract === "object" ? contractState.contract : null;
  const lifecycle = contract?.lifecycle && typeof contract.lifecycle === "object" ? contract.lifecycle : {};
  const packetText = fs.readFileSync(repoPathAbs(packetPath), "utf8");
  const contractPrefix = contract ? [
    `- PACKET_FORMAT_VERSION: ${lifecycle.packet_format_version || ""}`,
    `- MAIN_CONTAINMENT_STATUS: ${lifecycle.main_containment_status || ""}`,
    `- CURRENT_MAIN_COMPATIBILITY_STATUS: ${lifecycle.current_main_compatibility_status || ""}`,
  ].join("\n") : "";
  const result = validateMergeProgressionTruth(contractPrefix ? `${contractPrefix}\n${packetText}` : packetText, {
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
