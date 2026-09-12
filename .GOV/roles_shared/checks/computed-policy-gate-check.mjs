import fs from "node:fs";
import {
  computedPolicyOutcomeAllowsClosure,
  evaluateComputedPolicyGateFromPacketText,
  parsePolicyWaiverLedger,
} from "../scripts/lib/computed-policy-gate-lib.mjs";
import { GOV_ROOT_REPO_REL, listOfficialWorkPacketEntries, repoPathAbs, resolveWorkPacketPath } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { parsePacketSingleField } from "../scripts/lib/scope-surface-lib.mjs";
import { parseJsonlFile } from "../scripts/lib/wp-communications-lib.mjs";

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

function loadPacketReceipts(packetText = "") {
  const receiptsFile = parsePacketSingleField(packetText, "WP_RECEIPTS_FILE");
  if (!receiptsFile || !fs.existsSync(repoPathAbs(receiptsFile))) return [];
  return parseJsonlFile(receiptsFile);
}

const wpIdArg = process.argv[2] && !process.argv[2].startsWith("--") ? process.argv[2] : "";
const jsonMode = process.argv.includes("--json");
const waiversMode = process.argv.includes("--waivers");
const results = [];

// --waivers: print the resolved policy waiver ledger (markdown `WAIVERS GRANTED` or JSON
// `waivers_granted[]`) for the target packet(s) [VPX-006]; exit 1 when any entry is UNSIGNED.
if (waiversMode) {
  const ledgers = [];
  for (const target of loadTargetPackets(wpIdArg)) {
    const packetAbsPath = repoPathAbs(target.packetPath);
    if (!fs.existsSync(packetAbsPath)) fail("Work packet not found", [target.packetPath]);
    const ledger = parsePolicyWaiverLedger(fs.readFileSync(packetAbsPath, "utf8"));
    ledgers.push({ wpId: target.wpId, packetPath: target.packetPath, ledger });
  }
  const unsigned = ledgers.flatMap((entry) => entry.ledger.entries
    .filter((row) => row.status === "UNSIGNED")
    .map((row) => `${entry.wpId}: ${row.waiverId} is UNSIGNED (signature=${row.signature || "<missing>"})`));
  process.stdout.write(`${JSON.stringify(ledgers, null, 2)}
`);
  if (unsigned.length > 0) fail("Unsigned policy waivers found [VPX-006]", unsigned);
  process.exit(0);
}

for (const target of loadTargetPackets(wpIdArg)) {
  const packetAbsPath = repoPathAbs(target.packetPath);
  if (!fs.existsSync(packetAbsPath)) {
    fail("Work packet not found", [target.packetPath]);
  }
  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  const receipts = loadPacketReceipts(packetText);
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetText, {
    wpId: target.wpId,
    packetPath: target.packetPath,
    requireClosedStatus: true,
    receipts,
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
