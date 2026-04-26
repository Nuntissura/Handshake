const PRODUCT_CORRECTNESS = "PRODUCT_CORRECTNESS";
const GOVERNANCE_SUPPORT = "GOVERNANCE_SUPPORT";

const CLOSEOUT_DEPENDENCY_AUTHORITIES = Object.freeze({
  topology: GOVERNANCE_SUPPORT,
  closeout_bundle: GOVERNANCE_SUPPORT,
  scope_compatibility: PRODUCT_CORRECTNESS,
  candidate_target: PRODUCT_CORRECTNESS,
  sync_provenance: GOVERNANCE_SUPPORT,
  repomem_coverage: GOVERNANCE_SUPPORT,
});

const TERMINAL_NON_PASS_CLOSEOUT_MODES = new Set(["FAIL", "OUTDATED_ONLY", "ABANDONED"]);

function normalizeText(value, fallback = "") {
  const text = String(value || "").trim();
  return text || fallback;
}

export function normalizeCloseoutMode(mode = "") {
  return normalizeText(mode, "UNSET").toUpperCase();
}

export function isTerminalNonPassCloseoutMode(mode = "") {
  return TERMINAL_NON_PASS_CLOSEOUT_MODES.has(normalizeCloseoutMode(mode));
}

export function authorityClassForCloseoutDependency(key = "") {
  return CLOSEOUT_DEPENDENCY_AUTHORITIES[normalizeText(key).toLowerCase()] || GOVERNANCE_SUPPORT;
}

export function dependencyBlocksProductOutcome({
  key = "",
  required = false,
  status = "",
} = {}) {
  return Boolean(required)
    && normalizeText(status, "UNKNOWN").toUpperCase() === "FAIL"
    && authorityClassForCloseoutDependency(key) === PRODUCT_CORRECTNESS;
}

export function resolveArtifactHygieneCloseoutPolicy({
  closeoutMode = "",
} = {}) {
  const normalizedMode = normalizeCloseoutMode(closeoutMode);
  if (isTerminalNonPassCloseoutMode(normalizedMode)) {
    return {
      closeout_mode: normalizedMode,
      disposition: "SETTLEMENT_DEBT",
      debt_key: "ACTIVE_TOPOLOGY_ARTIFACT_HYGIENE",
      summary:
        "Active-topology artifact hygiene remains settlement debt for terminal non-pass closeout; it does not block product outcome once verdict-of-record exists.",
    };
  }
  return {
    closeout_mode: normalizedMode,
    disposition: "BLOCK",
    debt_key: "",
    summary:
      "Artifact hygiene remains settlement-blocking for closeout modes that still need clean publication promotion.",
  };
}
