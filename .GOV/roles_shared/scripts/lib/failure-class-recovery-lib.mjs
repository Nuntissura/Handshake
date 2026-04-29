const PRODUCT_PATTERNS = [
  /candidate[_ -]?target/i,
  /signed[_ -]?scope/i,
  /scope[_ -]?compatibility/i,
  /tripwire/i,
  /\bproof\b/i,
  /CLAUSE_CLOSURE|CLAUSE\b/i,
  /NOT_PROVEN/i,
];

const ENVIRONMENT_PATTERNS = [
  /artifact[_ -]?hygiene/i,
  /artifact[_ -]?root/i,
  /noncanonical/i,
  /cargo\s+target/i,
  /target[\\/]/i,
  /build cache/i,
  /local environment/i,
];

const GOVERNANCE_PATTERNS = [
  /dossier/i,
  /session/i,
  /runtime/i,
  /repomem/i,
  /projection/i,
  /closeout[_ -]?bundle/i,
  /sync[_ -]?provenance/i,
  /validator[_ -]?gate/i,
  /broker/i,
  /governance[_ -]?debt/i,
  /topology/i,
];

function normalizeTextItems(values = []) {
  return (Array.isArray(values) ? values : [values])
    .map((value) => String(value || "").trim())
    .filter(Boolean);
}

function collectFailureSignals({ dependencyView = null, issues = [], details = [] } = {}) {
  const blockingKeys = normalizeTextItems(dependencyView?.blocking_keys);
  const productKeys = normalizeTextItems(dependencyView?.product_outcome_blocking_keys);
  const governanceKeys = normalizeTextItems(dependencyView?.governance_debt_keys);
  const dependencySummaries = Object.entries(dependencyView?.dependencies || {})
    .flatMap(([key, value]) => [
      key,
      value?.status,
      value?.summary,
      ...(Array.isArray(value?.blockers) ? value.blockers : []),
    ]);
  return [
    ...blockingKeys,
    ...productKeys,
    ...governanceKeys,
    ...dependencySummaries,
    ...normalizeTextItems(issues),
    ...normalizeTextItems(details),
  ].filter(Boolean);
}

function anyMatch(items, patterns) {
  return items.some((item) => patterns.some((pattern) => pattern.test(item)));
}

export function classifyFailureRecovery({
  wpId = "WP-{ID}",
  dependencyView = null,
  issues = [],
  details = [],
} = {}) {
  const signals = collectFailureSignals({ dependencyView, issues, details });
  const productKeys = normalizeTextItems(dependencyView?.product_outcome_blocking_keys);
  const governanceKeys = normalizeTextItems(dependencyView?.governance_debt_keys);
  const environment = anyMatch(signals, ENVIRONMENT_PATTERNS);
  const product = productKeys.length > 0 || anyMatch(signals, PRODUCT_PATTERNS);
  const governance = governanceKeys.length > 0 || anyMatch(signals, GOVERNANCE_PATTERNS);

  if (product) {
    return {
      failure_class: "PRODUCT_BLOCKER",
      revalidation_required: true,
      product_proof_preserved: false,
      next_command: `just orchestrator-next ${wpId}`,
      reason: "Product evidence, signed scope, candidate target, or clause proof still blocks final PASS.",
      signals,
    };
  }

  if (environment) {
    return {
      failure_class: "ENVIRONMENT_BLOCKER",
      revalidation_required: false,
      product_proof_preserved: true,
      next_command: `just artifact-root-preflight ${wpId}`,
      reason: "Local artifact root or build-cache hygiene blocks mechanical closeout without invalidating product proof.",
      signals,
    };
  }

  if (governance) {
    return {
      failure_class: "GOVERNANCE_BLOCKER",
      revalidation_required: false,
      product_proof_preserved: true,
      next_command: `just closeout-repair ${wpId}`,
      reason: "Governance/session/runtime/dossier truth must be repaired before final closeout can continue.",
      signals,
    };
  }

  return {
    failure_class: "GOVERNANCE_BLOCKER",
    revalidation_required: false,
    product_proof_preserved: true,
    next_command: `just orchestrator-next ${wpId}`,
    reason: "Unclassified closeout failure defaults to governance recovery until a narrower blocker is recorded.",
    signals,
  };
}
