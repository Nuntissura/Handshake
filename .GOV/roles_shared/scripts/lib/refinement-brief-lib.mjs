function formatList(items, { indent = '  - ', none = 'NONE' } = {}) {
  const normalized = (items || []).map((item) => String(item || '').trim()).filter(Boolean);
  if (normalized.length === 0) return `${indent}${none}`;
  return normalized.map((item) => `${indent}${item}`).join('\n');
}

function normalizeExcerpt(excerpt) {
  return String(excerpt || '').replace(/\r/g, '').trim();
}

export function formatSpecContextWindowsSection(specAnchors) {
  const anchors = (specAnchors || []).filter((anchor) => anchor && (
    String(anchor.specAnchor || '').trim()
    || String(anchor.contextToken || '').trim()
    || String(anchor.excerpt || '').trim()
  ));

  const anchorsBlock = anchors.length === 0
    ? '- NONE'
    : anchors.map((anchor, idx) => `#### ANCHOR ${idx + 1}
- SPEC_ANCHOR: ${String(anchor.specAnchor || '').trim() || '<missing>'}
- CONTEXT_START_LINE: ${String(anchor.contextStartLine || '').trim() || '<missing>'}
- CONTEXT_END_LINE: ${String(anchor.contextEndLine || '').trim() || '<missing>'}
- CONTEXT_TOKEN: ${String(anchor.contextToken || '').trim() || '<missing>'}
- EXCERPT_ASCII_ESCAPED:
  \`\`\`text
${normalizeExcerpt(anchor.excerpt)}
  \`\`\``).join('\n\n');

  return `## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
${anchorsBlock}`;
}

export function formatClauseProofPlanSection(clauseProofPlan) {
  return `## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
${formatList(clauseProofPlan)}`;
}

export function formatContractSurfacesSection(contractSurfaces) {
  return `## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
${formatList(contractSurfaces)}`;
}

export function formatSemanticProofAssetsSection({
  semanticTripwireTests,
  canonicalContractExamples,
}) {
  return `## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
${formatList(semanticTripwireTests)}
- CANONICAL_CONTRACT_EXAMPLES:
${formatList(canonicalContractExamples)}
- Rule: for packets using \`SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1\`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.`;
}

export function formatCoderHandoffBriefSection({
  implementationOrder,
  hotFiles,
  tripwireTests,
  carryForwardWarnings,
}) {
  return `## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
${formatList(implementationOrder)}
- HOT_FILES:
${formatList(hotFiles)}
- TRIPWIRE_TESTS:
${formatList(tripwireTests)}
- CARRY_FORWARD_WARNINGS:
${formatList(carryForwardWarnings)}`;
}

export function formatValidatorHandoffBriefSection({
  clausesToInspect,
  filesToRead,
  commandsToRun,
  postMergeSpotchecks,
}) {
  return `## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
${formatList(clausesToInspect)}
- FILES_TO_READ:
${formatList(filesToRead)}
- COMMANDS_TO_RUN:
${formatList(commandsToRun)}
- POST_MERGE_SPOTCHECKS:
${formatList(postMergeSpotchecks)}`;
}

export function formatNotProvenAtRefinementTimeSection(refinementNotProven) {
  return `## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
${formatList(refinementNotProven)}`;
}
