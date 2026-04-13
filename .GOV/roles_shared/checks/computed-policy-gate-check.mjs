import fs from "node:fs";
import {
  computedPolicyOutcomeAllowsClosure,
  evaluateComputedPolicyGateFromPacketText,
} from "../scripts/lib/computed-policy-gate-lib.mjs";
import { GOV_ROOT_REPO_REL, listOfficialWorkPacketEntries, repoPathAbs, resolveWorkPacketPath } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("computed-policy-gate-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("computed-policy-gate-check.mjs", message, { role: "SHARED", details });
}

function summarizeIssues(items) {
  return items.map((item) => `${item.code}: ${item.message}`);
}

function loadTargetPackets(wpId) {
  if (wpId) {
    const resolved = resolveWorkPacketPath(wpId);
    const packetPath = resolved?.packetPath || `${GOV_ROOT_REPO_REL}/task_packets/${wpId}.md`;
    return [{ wpId, packetPath }];
  }

  return listOfficialWorkPacketEntries().map(({ wpId, packetPath }) => ({ wpId, packetPath }));
}

const wpIdArg = process.argv[2] && !process.argv[2].startsWith("--") ? process.argv[2] : "";
const jsonMode = process.argv.includes("--json");
const results = [];

for (const target of loadTargetPackets(wpIdArg)) {
  const packetAbsPath = repoPathAbs(target.packetPath);
  if (!fs.existsSync(packetAbsPath)) {
    fail("Work packet not found", [target.packetPath]);
  }
  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetText, {
    wpId: target.wpId,
    packetPath: target.packetPath,
    requireClosedStatus: true,
  });
  if (!evaluation.applicable && !evaluation.legacy_remediation_required) continue;
  results.push({
    wpId: target.wpId,
    packetPath: target.packetPath,
    evaluation,
  });
}

if (jsonMode) {
  process.stdout.write(`${JSON.stringify(results, null, 2)}\n`);
  process.exit(results.every((entry) => computedPolicyOutcomeAllowsClosure(entry.evaluation)) ? 0 : 1);
}

const failures = results.filter((entry) => !computedPolicyOutcomeAllowsClosure(entry.evaluation));
if (failures.length > 0) {
  const details = [];
  for (const failure of failures) {
    details.push(`${failure.wpId}: outcome=${failure.evaluation.outcome}`);
    details.push(...summarizeIssues(failure.evaluation.issues.fail));
    details.push(...summarizeIssues(failure.evaluation.issues.blocked));
    details.push(...summarizeIssues(failure.evaluation.issues.reviewRequired));
  }
  fail("Computed policy gate violations found", details);
}

for (const result of results) {
  const suffix = result.evaluation.outcome === "WAIVED" ? " (waived)" : "";
  console.log(`[COMPUTED_POLICY_GATE] PASS: ${result.wpId} => ${result.evaluation.outcome}${suffix}`);
}
if (results.length === 0) {
  console.log("[COMPUTED_POLICY_GATE] PASS: no applicable closed packets");
}
