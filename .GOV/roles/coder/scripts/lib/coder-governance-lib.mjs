import { evaluateComputedPolicyGateFromPacketText } from "../../../../roles_shared/scripts/lib/computed-policy-gate-lib.mjs";

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

export function evaluateCoderPacketGovernanceState({
  wpId = "",
  packetPath = "",
  packetContent = "",
  currentWpStatus = "",
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

  if (hasValidatorBoundaryStatus(currentWpStatus)) {
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
    terminalReason: "ACTIVE",
    packetStatus,
    currentWpStatus,
    computedPolicy,
    message: "Packet remains coder-resumable under current governance state.",
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
