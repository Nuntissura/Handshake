import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  DEFAULT_BASELINE_REF_CANDIDATES,
  deriveCoderResumeState,
  evaluateCoderPacketGovernanceState,
  loadCoderCommunicationState,
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

test("coder communication state loads runtime and latest validator assessment from packet-declared ledgers", async (t) => {
  const repoRoot = path.resolve(".");
  const tempDir = path.join(repoRoot, ".tmp-coder-communication-state");
  await fs.promises.rm(tempDir, { recursive: true, force: true });
  await fs.promises.mkdir(tempDir, { recursive: true });
  t.after(async () => {
    await fs.promises.rm(tempDir, { recursive: true, force: true });
  });

  const runtimeFile = path.join(tempDir, "runtime.json");
  const receiptsFile = path.join(tempDir, "receipts.jsonl");
  await fs.promises.writeFile(runtimeFile, JSON.stringify({
    next_expected_actor: "CODER",
    waiting_on: "CODER_REPAIR_HANDOFF",
  }, null, 2));
  await fs.promises.writeFile(receiptsFile, [
    JSON.stringify({
      timestamp_utc: "2026-04-01T10:00:00Z",
      wp_id: "WP-TEST-CODER-v1",
      actor_role: "WP_VALIDATOR",
      actor_session: "wpv-test",
      receipt_kind: "VALIDATOR_REVIEW",
      summary: "Repair required. Fix the projection and re-handoff.",
      target_role: "CODER",
      target_session: "coder-test",
      correlation_id: "corr-1",
      ack_for: "corr-1",
      requires_ack: false,
      refs: [".GOV/task_packets/WP-TEST-CODER-v1/packet.md"],
    }),
  ].join("\n"));

  const state = loadCoderCommunicationState({
    wpId: "WP-TEST-CODER-v1",
    packetPath: ".GOV/task_packets/WP-TEST-CODER-v1/packet.md",
    packetContent: `# Task Packet: WP-TEST-CODER-v1

**Status:** In Progress

## METADATA
- WP_ID: WP-TEST-CODER-v1
- PACKET_FORMAT_VERSION: 2026-03-29
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
- COMMUNICATION_HEALTH_GATE: DIRECT_REVIEW_REQUIRED
- WP_RUNTIME_STATUS_FILE: ${path.relative(repoRoot, runtimeFile).replace(/\\/g, "/")}
- WP_RECEIPTS_FILE: ${path.relative(repoRoot, receiptsFile).replace(/\\/g, "/")}
`.trim(),
  });

  assert.equal(state.runtimeStatus.next_expected_actor, "CODER");
  assert.equal(state.communicationEvaluation.applicable, true);
  assert.equal(state.latestValidatorAssessment?.verdict, "FAIL");
  assert.match(state.latestValidatorAssessment?.reason || "", /Repair required/i);
});

test("coder resume state surfaces remediation when runtime routes back after validator fail", () => {
  const state = deriveCoderResumeState({
    communicationState: {
      communicationEvaluation: { applicable: true },
      runtimeStatus: {
        next_expected_actor: "CODER",
        waiting_on: "CODER_REPAIR_HANDOFF",
      },
      latestValidatorAssessment: {
        verdict: "FAIL",
        receiptKind: "VALIDATOR_REVIEW",
        reason: "Repair required. Commit the reviewable state and re-handoff.",
      },
    },
  });

  assert.equal(state.ready, true);
  assert.equal(state.blockedByRoute, false);
  assert.equal(state.remediationRequired, true);
  assert.equal(state.nextExpectedActor, "CODER");
  assert.match(state.message, /recorded FAIL/i);
});

test("coder resume state blocks when validator review is still the routed next actor", () => {
  const state = deriveCoderResumeState({
    communicationState: {
      communicationEvaluation: { applicable: true },
      runtimeStatus: {
        next_expected_actor: "WP_VALIDATOR",
        waiting_on: "WP_VALIDATOR_REVIEW",
      },
      latestValidatorAssessment: null,
    },
  });

  assert.equal(state.ready, false);
  assert.equal(state.blockedByRoute, true);
  assert.equal(state.remediationRequired, false);
  assert.match(state.message, /WP validator review is next/i);
});
