export const GOVERNED_VALIDATOR_REPORT_PROFILE_VALUES = Object.freeze([
  "SPLIT_DIFF_SCOPED_V1",
  "SPLIT_DIFF_SCOPED_RIGOR_V2",
  "SPLIT_DIFF_SCOPED_RIGOR_V3",
  "SPLIT_DIFF_SCOPED_RIGOR_V4",
]);
export const DUAL_TRACK_VALIDATOR_MIN_VERSION = "2026-04-05";

export function normalizeValidatorReportProfile(value) {
  const normalized = String(value || "").trim().toUpperCase();
  return GOVERNED_VALIDATOR_REPORT_PROFILE_VALUES.includes(normalized)
    ? normalized
    : "";
}

export function validatorReportProfileUsesHeuristicRigor(value) {
  const profile = normalizeValidatorReportProfile(value);
  return profile === "SPLIT_DIFF_SCOPED_RIGOR_V2"
    || profile === "SPLIT_DIFF_SCOPED_RIGOR_V3"
    || profile === "SPLIT_DIFF_SCOPED_RIGOR_V4";
}

export function validatorReportProfileRequiresRiskAudit(value) {
  const profile = normalizeValidatorReportProfile(value);
  return profile === "SPLIT_DIFF_SCOPED_RIGOR_V3"
    || profile === "SPLIT_DIFF_SCOPED_RIGOR_V4";
}

export function validatorReportProfileRequiresPrimitiveAudit(value) {
  return normalizeValidatorReportProfile(value) === "SPLIT_DIFF_SCOPED_RIGOR_V4";
}

export function validatorReportProfileRequiresAntiVibe(value, packetFormatVersion = "") {
  return validatorReportProfileRequiresRiskAudit(value)
    && String(packetFormatVersion || "").trim() >= "2026-04-01";
}

export function validatorReportProfileRequiresDualTrack(
  value,
  packetFormatVersion = "",
  packetRiskTier = "",
) {
  const normalizedRiskTier = String(packetRiskTier || "").trim().toUpperCase();
  return validatorReportProfileRequiresPrimitiveAudit(value)
    && String(packetFormatVersion || "").trim() >= DUAL_TRACK_VALIDATOR_MIN_VERSION
    && (normalizedRiskTier === "MEDIUM" || normalizedRiskTier === "HIGH");
}
