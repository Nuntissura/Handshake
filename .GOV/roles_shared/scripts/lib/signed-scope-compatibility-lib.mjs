const RFC3339_UTC_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/;
const SHA_RE = /^[0-9a-f]{40}$/i;

export const SIGNED_SCOPE_COMPATIBILITY_PACKET_MIN_VERSION = "2026-03-26";
export const CURRENT_MAIN_COMPATIBILITY_STATUS_VALUES = [
  "NOT_RUN",
  "COMPATIBLE",
  "ADJACENT_SCOPE_REQUIRED",
  "BLOCKED",
];
export const PACKET_WIDENING_DECISION_VALUES = [
  "NONE",
  "NOT_REQUIRED",
  "FOLLOW_ON_WP_REQUIRED",
  "SUPERSEDING_PACKET_REQUIRED",
];

function parseSingleField(packetText, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(packetText || "").match(re);
  return match ? match[1].trim() : "";
}

function replaceSingleField(packetText, label, nextValue) {
  const re = new RegExp(`^(\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*)(.+)\\s*$`, "mi");
  if (!re.test(String(packetText || ""))) {
    throw new Error(`Missing packet field: ${label}`);
  }
  return String(packetText || "").replace(re, `$1${nextValue}`);
}

function isNoneLike(value) {
  const normalized = String(value || "").trim().toUpperCase();
  return normalized === "NONE" || normalized === "N/A";
}

export function packetRequiresSignedScopeCompatibility(packetFormatVersion) {
  return String(packetFormatVersion || "").trim() >= SIGNED_SCOPE_COMPATIBILITY_PACKET_MIN_VERSION;
}

export function parseSignedScopeCompatibilityTruth(packetText) {
  return {
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
    currentMainCompatibilityStatus: String(parseSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_STATUS") || "").trim().toUpperCase(),
    currentMainCompatibilityBaselineSha: String(parseSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA") || "").trim(),
    currentMainCompatibilityVerifiedAtUtc: String(parseSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC") || "").trim(),
    packetWideningDecision: String(parseSingleField(packetText, "PACKET_WIDENING_DECISION") || "").trim().toUpperCase(),
    packetWideningEvidence: String(parseSingleField(packetText, "PACKET_WIDENING_EVIDENCE") || "").trim(),
  };
}

export function updateSignedScopeCompatibilityTruth(packetText, {
  currentMainCompatibilityStatus,
  currentMainCompatibilityBaselineSha,
  currentMainCompatibilityVerifiedAtUtc,
  packetWideningDecision,
  packetWideningEvidence,
} = {}) {
  let nextText = String(packetText || "");
  if (currentMainCompatibilityStatus != null) {
    nextText = replaceSingleField(nextText, "CURRENT_MAIN_COMPATIBILITY_STATUS", currentMainCompatibilityStatus);
  }
  if (currentMainCompatibilityBaselineSha != null) {
    nextText = replaceSingleField(nextText, "CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA", currentMainCompatibilityBaselineSha);
  }
  if (currentMainCompatibilityVerifiedAtUtc != null) {
    nextText = replaceSingleField(nextText, "CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC", currentMainCompatibilityVerifiedAtUtc);
  }
  if (packetWideningDecision != null) {
    nextText = replaceSingleField(nextText, "PACKET_WIDENING_DECISION", packetWideningDecision);
  }
  if (packetWideningEvidence != null) {
    nextText = replaceSingleField(nextText, "PACKET_WIDENING_EVIDENCE", packetWideningEvidence);
  }
  return nextText;
}

export function validateSignedScopeCompatibilityTruth(packetText, {
  packetPath = "",
  currentMainHeadSha = "",
  requireReadyForPass = false,
} = {}) {
  const parsed = parseSignedScopeCompatibilityTruth(packetText);
  const errors = [];

  if (!packetRequiresSignedScopeCompatibility(parsed.packetFormatVersion)) {
    return { parsed, errors };
  }

  if (!CURRENT_MAIN_COMPATIBILITY_STATUS_VALUES.includes(parsed.currentMainCompatibilityStatus)) {
    errors.push(
      `${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS must be ${CURRENT_MAIN_COMPATIBILITY_STATUS_VALUES.join(" | ")} (got ${parsed.currentMainCompatibilityStatus || "<missing>"})`,
    );
  }
  if (!PACKET_WIDENING_DECISION_VALUES.includes(parsed.packetWideningDecision)) {
    errors.push(
      `${packetPath || "<packet>"}: PACKET_WIDENING_DECISION must be ${PACKET_WIDENING_DECISION_VALUES.join(" | ")} (got ${parsed.packetWideningDecision || "<missing>"})`,
    );
  }

  const status = parsed.currentMainCompatibilityStatus;
  const widening = parsed.packetWideningDecision;
  const baseline = parsed.currentMainCompatibilityBaselineSha;
  const verifiedAt = parsed.currentMainCompatibilityVerifiedAtUtc;
  const evidence = parsed.packetWideningEvidence;
  const normalizedCurrentMainHeadSha = String(currentMainHeadSha || "").trim();

  if (status === "NOT_RUN") {
    if (!isNoneLike(baseline)) {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN requires CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA=NONE`);
    }
    if (!/^N\/A$/i.test(verifiedAt)) {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN requires CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC=N/A`);
    }
    if (widening !== "NONE") {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN requires PACKET_WIDENING_DECISION=NONE`);
    }
    if (!/^N\/A$/i.test(evidence)) {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN requires PACKET_WIDENING_EVIDENCE=N/A`);
    }
    if (requireReadyForPass) {
      errors.push(`${packetPath || "<packet>"}: PASS-ready closeout requires CURRENT_MAIN_COMPATIBILITY_STATUS=COMPATIBLE, not NOT_RUN`);
    }
    return { parsed, errors };
  }

  if (!SHA_RE.test(baseline)) {
    errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA must be a full commit sha when CURRENT_MAIN_COMPATIBILITY_STATUS is ${status || "<missing>"}`);
  }
  if (!RFC3339_UTC_RE.test(verifiedAt)) {
    errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC must be RFC3339 UTC when CURRENT_MAIN_COMPATIBILITY_STATUS is ${status || "<missing>"}`);
  }
  if (normalizedCurrentMainHeadSha && SHA_RE.test(baseline) && baseline.toLowerCase() !== normalizedCurrentMainHeadSha.toLowerCase()) {
    errors.push(
      `${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA (${baseline}) does not match current local main HEAD (${normalizedCurrentMainHeadSha})`,
    );
  }

  if (status === "COMPATIBLE") {
    if (widening !== "NOT_REQUIRED") {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=COMPATIBLE requires PACKET_WIDENING_DECISION=NOT_REQUIRED`);
    }
    if (!/^N\/A$/i.test(evidence)) {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=COMPATIBLE requires PACKET_WIDENING_EVIDENCE=N/A`);
    }
  } else if (status === "ADJACENT_SCOPE_REQUIRED") {
    if (!["FOLLOW_ON_WP_REQUIRED", "SUPERSEDING_PACKET_REQUIRED"].includes(widening)) {
      errors.push(
        `${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=ADJACENT_SCOPE_REQUIRED requires PACKET_WIDENING_DECISION=FOLLOW_ON_WP_REQUIRED|SUPERSEDING_PACKET_REQUIRED`,
      );
    }
    if (isNoneLike(evidence)) {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=ADJACENT_SCOPE_REQUIRED requires concrete PACKET_WIDENING_EVIDENCE`);
    }
    if (requireReadyForPass) {
      errors.push(`${packetPath || "<packet>"}: PASS-ready closeout prohibited when CURRENT_MAIN_COMPATIBILITY_STATUS=ADJACENT_SCOPE_REQUIRED`);
    }
  } else if (status === "BLOCKED") {
    if (widening !== "NONE") {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=BLOCKED requires PACKET_WIDENING_DECISION=NONE`);
    }
    if (!/^N\/A$/i.test(evidence)) {
      errors.push(`${packetPath || "<packet>"}: CURRENT_MAIN_COMPATIBILITY_STATUS=BLOCKED requires PACKET_WIDENING_EVIDENCE=N/A`);
    }
    if (requireReadyForPass) {
      errors.push(`${packetPath || "<packet>"}: PASS-ready closeout prohibited when CURRENT_MAIN_COMPATIBILITY_STATUS=BLOCKED`);
    }
  }

  return { parsed, errors };
}
