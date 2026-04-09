import fs from "node:fs";
import path from "node:path";
import { evaluateComputedPolicyGateFromPacketText } from "../../../../roles_shared/scripts/lib/computed-policy-gate-lib.mjs";
import { parseClaimField } from "../../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { evaluateWpCommunicationHealth, deriveLatestValidatorAssessment } from "../../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs } from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";

export const DEFAULT_BASELINE_REF_CANDIDATES = ["main", "origin/main", "gov_kernel", "origin/gov_kernel"];

function parseStatus(packetContent) {
  return (
    (String(packetContent || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(packetContent || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(packetContent || "").match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1]
    || ""
  ).trim();
}

function hasClosedPacketStatus(status) {
  return /\b(done|validated)\b/i.test(String(status || ""));
}

function hasHistoricalPacketMarker(status) {
  return /\b(historical|outdated|superseded|fail|failed)\b/i.test(String(status || ""));
}

function hasValidatorBoundaryStatus(status) {
  return /\b(done|validated|validator|validation|handoff|fail|failed|outdated|superseded)\b/i.test(String(status || ""));
}

function repoRelativeFileExists(filePath = "") {
  const relPath = String(filePath || "").trim();
  if (!relPath) return false;
  try {
    return path.isAbsolute(relPath) ? false : fs.existsSync(repoPathAbs(relPath));
  } catch {
    return false;
  }
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function coderReadyMessage(waitingOn, communicationState = null) {
  const waiting = String(waitingOn || "").trim().toUpperCase();
  const latestAssessment = communicationState?.latestValidatorAssessment || null;
  if (latestAssessment?.verdict === "FAIL") {
    return `Latest validator assessment already recorded FAIL; coder remediation is next (${waiting || "CODER_REPAIR_HANDOFF"}).`;
  }
  if (waiting === "CODER_INTENT") {
    return "WP validator kickoff is open; coder intent reply is required now.";
  }
  if (waiting === "CODER_HANDOFF") {
    return "Kickoff exchange is complete; coder handoff to WP validator is required now.";
  }
  if (waiting === "FINAL_REVIEW_EXCHANGE") {
    return "Coder must initiate the direct final review exchange with Integration Validator.";
  }
  if (waiting.startsWith("OPEN_REVIEW_ITEM_")) {
    return "Open review traffic is targeted to coder and requires a response.";
  }
  return "Coder is the projected next actor for the current governed step.";
}

function blockedCoderMessage(nextExpectedActor, waitingOn, communicationState = null) {
  const nextActor = normalizeRole(nextExpectedActor);
  const waiting = String(waitingOn || "").trim();
  const latestAssessment = communicationState?.latestValidatorAssessment || null;

  if (nextActor === "WP_VALIDATOR") {
    if (String(waiting || "").trim().toUpperCase() === "WP_VALIDATOR_INTENT_CHECKPOINT") {
      return "Coder intent is recorded; wait for WP validator checkpoint clearance before implementation or full handoff.";
    }
    return `Coder handoff is already recorded; WP validator review is next (${waiting || "WP_VALIDATOR_REVIEW"}).`;
  }
  if (nextActor === "INTEGRATION_VALIDATOR") {
    return `WP validator progression already cleared the final lane; Integration Validator is next (${waiting || "FINAL_REVIEW_EXCHANGE"}).`;
  }
  if (nextActor === "ORCHESTRATOR") {
    if (latestAssessment) {
      return `Latest validator assessment already recorded ${latestAssessment.verdict}; orchestrator progression is next (${waiting || "VERDICT_PROGRESSION"}).`;
    }
    return `Coder work is not the current route target; runtime expects ORCHESTRATOR next (${waiting || "governance"}).`;
  }
  if (nextActor) {
    return `Coder work is not the current route target; runtime expects ${nextActor} next (${waiting || "governed progression"}).`;
  }
  return "Coder work is not the current route target yet.";
}

function runtimeRoutesCoderFinalReview(communicationState = null) {
  return normalizeRole(communicationState?.runtimeStatus?.next_expected_actor) === "CODER"
    && String(communicationState?.runtimeStatus?.waiting_on || "").trim().toUpperCase() === "FINAL_REVIEW_EXCHANGE";
}

export function evaluateCoderPacketGovernanceState({
  wpId = "",
  packetPath = "",
  packetContent = "",
  currentWpStatus = "",
  communicationState = null,
} = {}) {
  const packetStatus = parseStatus(packetContent);
  const computedPolicy = evaluateComputedPolicyGateFromPacketText(packetContent, {
    wpId,
    packetPath,
    requireClosedStatus: true,
  });

  if (computedPolicy.legacy_remediation_required) {
    const blockedMessage = computedPolicy.issues.blocked[0]?.message
      || "Closed structured packet requires remediation in a newer packet revision.";
    return {
      allowResume: false,
      legacyRemediationRequired: true,
      terminalReason: "LEGACY_REMEDIATION_REQUIRED",
      packetStatus,
      currentWpStatus,
      computedPolicy,
      message: blockedMessage,
    };
  }

  if (hasHistoricalPacketMarker(packetStatus) || hasClosedPacketStatus(packetStatus)) {
    return {
      allowResume: false,
      legacyRemediationRequired: false,
      terminalReason: "CLOSED_PACKET_STATUS",
      packetStatus,
      currentWpStatus,
      computedPolicy,
      message: `Packet status is "${packetStatus || "<missing>"}"; coder must not resume implementation on a closed packet.`,
    };
  }

  if (hasValidatorBoundaryStatus(currentWpStatus) && !runtimeRoutesCoderFinalReview(communicationState)) {
    return {
      allowResume: false,
      legacyRemediationRequired: false,
      terminalReason: "VALIDATOR_HANDOFF",
      packetStatus,
      currentWpStatus,
      computedPolicy,
      message: `Current WP_STATUS is "${currentWpStatus || "<missing>"}"; coder must not resume while validator/handoff state is active.`,
    };
  }

  return {
    allowResume: true,
    legacyRemediationRequired: false,
    terminalReason: runtimeRoutesCoderFinalReview(communicationState) ? "ACTIVE_FINAL_REVIEW" : "ACTIVE",
    packetStatus,
    currentWpStatus,
    computedPolicy,
    message: runtimeRoutesCoderFinalReview(communicationState)
      ? "Packet remains coder-resumable for the routed final review exchange."
      : "Packet remains coder-resumable under current governance state.",
  };
}

export function loadCoderCommunicationState({
  wpId = "",
  packetPath = "",
  packetContent = "",
} = {}) {
  const runtimeStatusFile = String(parseClaimField(packetContent, "WP_RUNTIME_STATUS_FILE") || "").trim();
  const receiptsFile = String(parseClaimField(packetContent, "WP_RECEIPTS_FILE") || "").trim();
  if (!runtimeStatusFile || !repoRelativeFileExists(runtimeStatusFile)) return null;

  const runtimeStatus = parseJsonFile(runtimeStatusFile);
  const receipts = receiptsFile && repoRelativeFileExists(receiptsFile) ? parseJsonlFile(receiptsFile) : [];
  const communicationEvaluation = evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath,
    packetContent,
    workflowLane: parseClaimField(packetContent, "WORKFLOW_LANE"),
    packetFormatVersion: parseClaimField(packetContent, "PACKET_FORMAT_VERSION"),
    communicationContract: parseClaimField(packetContent, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: parseClaimField(packetContent, "COMMUNICATION_HEALTH_GATE"),
    receipts,
    runtimeStatus,
  });

  return {
    runtimeStatus,
    receipts,
    communicationEvaluation,
    latestValidatorAssessment: deriveLatestValidatorAssessment(receipts),
  };
}

export function deriveCoderResumeState({
  communicationState = null,
} = {}) {
  const nextExpectedActor = normalizeRole(communicationState?.runtimeStatus?.next_expected_actor);
  const waitingOn = String(communicationState?.runtimeStatus?.waiting_on || "").trim();
  const latestAssessment = communicationState?.latestValidatorAssessment || null;
  const communicationApplicable = Boolean(communicationState?.communicationEvaluation?.applicable);

  if (!communicationApplicable) {
    return {
      ready: false,
      blockedByRoute: false,
      remediationRequired: false,
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: "",
    };
  }

  if (nextExpectedActor === "CODER") {
    return {
      ready: true,
      blockedByRoute: false,
      remediationRequired: latestAssessment?.verdict === "FAIL" || waitingOn === "CODER_REPAIR_HANDOFF",
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: coderReadyMessage(waitingOn, communicationState),
    };
  }

  if (nextExpectedActor) {
    return {
      ready: false,
      blockedByRoute: true,
      remediationRequired: false,
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: blockedCoderMessage(nextExpectedActor, waitingOn, communicationState),
    };
  }

  return {
    ready: false,
    blockedByRoute: false,
    remediationRequired: false,
    nextExpectedActor,
    waitingOn,
    latestAssessment,
    message: "",
  };
}

export function resolveGitBaselineMergeBase(runGitTrim, {
  headRef = "HEAD",
  candidateRefs = DEFAULT_BASELINE_REF_CANDIDATES,
} = {}) {
  for (const ref of candidateRefs) {
    try {
      const base = String(runGitTrim(`git merge-base ${ref} ${headRef}`) || "").trim();
      if (base) {
        return { base, ref };
      }
    } catch {
      // Ignore unavailable refs and continue to the next baseline candidate.
    }
  }
  return { base: null, ref: null };
}
