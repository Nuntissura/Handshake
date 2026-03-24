import assert from "node:assert/strict";
import test from "node:test";
import {
  DEFAULT_BASELINE_REF_CANDIDATES,
  evaluateCoderPacketGovernanceState,
  resolveGitBaselineMergeBase,
} from "../scripts/lib/coder-governance-lib.mjs";

function packetFixture({
  packetFormatVersion = "2026-03-23",
  status = "In Progress",
} = {}) {
  return `# Task Packet: WP-TEST-CODER-v1

**Status:** ${status}

## METADATA
- WP_ID: WP-TEST-CODER-v1
- PACKET_FORMAT_VERSION: ${packetFormatVersion}
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
`.trim();
}

test("coder packet policy blocks pre-threshold folder packets as remediation-required legacy closures", () => {
  const evaluation = evaluateCoderPacketGovernanceState({
    wpId: "WP-TEST-CODER-v1",
    packetPath: ".GOV/task_packets/WP-TEST-CODER-v1/packet.md",
    packetContent: packetFixture({
      packetFormatVersion: "2026-03-18",
      status: "Done",
    }),
    currentWpStatus: "Done",
  });

  assert.equal(evaluation.allowResume, false);
  assert.equal(evaluation.legacyRemediationRequired, true);
  assert.equal(evaluation.terminalReason, "LEGACY_REMEDIATION_REQUIRED");
});

test("coder packet policy stops resume on closed packet status even without legacy remediation", () => {
  const evaluation = evaluateCoderPacketGovernanceState({
    wpId: "WP-TEST-CODER-v1",
    packetPath: ".GOV/task_packets/WP-TEST-CODER-v1/packet.md",
    packetContent: packetFixture({
      packetFormatVersion: "2026-03-23",
      status: "Done",
    }),
    currentWpStatus: "",
  });

  assert.equal(evaluation.allowResume, false);
  assert.equal(evaluation.legacyRemediationRequired, false);
  assert.equal(evaluation.terminalReason, "CLOSED_PACKET_STATUS");
});

test("merge-base resolution falls back across baseline refs without assuming local main exists", () => {
  const calls = [];
  const evaluation = resolveGitBaselineMergeBase((command) => {
    calls.push(command);
    if (command === "git merge-base gov_kernel HEAD") {
      return "abc123";
    }
    throw new Error("missing ref");
  });

  assert.deepEqual(calls, [
    "git merge-base main HEAD",
    "git merge-base origin/main HEAD",
    "git merge-base gov_kernel HEAD",
  ]);
  assert.deepEqual(DEFAULT_BASELINE_REF_CANDIDATES, ["main", "origin/main", "gov_kernel", "origin/gov_kernel"]);
  assert.deepEqual(evaluation, { base: "abc123", ref: "gov_kernel" });
});
