import { loadSpecDebtRegistry } from './spec-debt-registry-lib.mjs';

function formatList(items, { indent = '  - ', none = 'NONE' } = {}) {
  const normalized = (items || []).map((item) => String(item || '').trim()).filter(Boolean);
  if (normalized.length === 0) return `${indent}${none}`;
  return normalized.map((item) => `${indent}${item}`).join('\n');
}

function parsePipeRecord(item) {
  const record = {};
  for (const part of String(item || '').split('|')) {
    const trimmed = part.trim();
    if (!trimmed) continue;
    const idx = trimmed.indexOf(':');
    if (idx === -1) continue;
    const key = trimmed.slice(0, idx).trim().toUpperCase().replace(/\s+/g, '_');
    const value = trimmed.slice(idx + 1).trim();
    record[key] = value;
  }
  return record;
}

function extractSectionAfterHeading(text, heading) {
  const lines = String(text || '').split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${heading}\\b`, 'i');
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) return '';

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }
  return lines.slice(startIndex, endIndex).join('\n').trim();
}

function replaceSection(text, heading, replacement) {
  const lines = String(text || '').split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${heading}\\b`, 'i');
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) {
    throw new Error(`Missing packet section heading: ${heading}`);
  }

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }

  const replacementLines = String(replacement || '').replace(/\r/g, '').split('\n');
  return [
    ...lines.slice(0, startIndex),
    ...replacementLines,
    ...lines.slice(endIndex),
  ].join('\n');
}

function extractListItemsAfterLabel(sectionText, label) {
  const lines = String(sectionText || '').split(/\r?\n/);
  const labelRe = new RegExp(`^(?:\\s*#{1,6}\\s+|\\s*-\\s*|\\s*)(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*$`, 'i');
  const headingRe = /^#{1,6}\s+\S/;
  const nextLabelRe = /^(?:\s*-\s*|\s*)[A-Z][A-Z0-9_ ()/.-]*(?:\*\*)?\s*:\s*$/;
  // Collect items from ALL occurrences (reports are append-only; clause text
  // may appear in any report, not just the most recent one).
  const items = [];
  for (let i = 0; i < lines.length; i += 1) {
    if (!labelRe.test(lines[i])) continue;
    for (let index = i + 1; index < lines.length; index += 1) {
      const line = lines[index];
      if (headingRe.test(line)) break;
      if (nextLabelRe.test(line)) break;
      const match = line.match(/^\s*-\s+(.+)\s*$/);
      if (match) items.push((match[1] || '').trim());
    }
  }
  return items;
}

function parseSingleField(sectionText, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, 'mi');
  const match = String(sectionText || '').match(re);
  return match ? match[1].trim() : '';
}

const SHARED_SURFACE_PATTERNS = [
  /\/workflows\.rs$/i,
  /\/storage\//i,
  /\/migrations\//i,
  /\/api\//i,
  /\/locus\//i,
  /\/mex\//i,
  /\/mcp\//i,
  /\/runtime/i,
  /\/types\.rs$/i,
  /\/schema/i,
];

function normalizeInlineBlock(value, fallback = 'See TEST_PLAN') {
  const collapsed = String(value || '').replace(/\r/g, '').split('\n').map((line) => line.trim()).filter((line) => line && !line.startsWith('#')).join('; ');
  return collapsed || fallback;
}

function uniqueOrdered(items) {
  const seen = new Set();
  const result = [];
  for (const item of items || []) {
    const normalized = String(item || '').trim();
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    result.push(normalized);
  }
  return result;
}

function parseValidationReportsSection(packetText) {
  const reportsSection = extractSectionAfterHeading(packetText, 'VALIDATION_REPORTS');
  return {
    raw: reportsSection,
    clausesReviewed: extractListItemsAfterLabel(reportsSection, 'CLAUSES_REVIEWED').filter((item) => !/^NONE$/i.test(item || '')),
    notProven: extractListItemsAfterLabel(reportsSection, 'NOT_PROVEN').filter((item) => !/^NONE$/i.test(item || '')),
    specAlignmentVerdict: (() => {
      const re = /^(?:\s*-\s*|\s*#{1,6}\s+|\s*)SPEC_ALIGNMENT_VERDICT\s*:\s*(.+)\s*$/gim;
      const matches = [...reportsSection.matchAll(re)];
      const match = matches.length > 0 ? matches[matches.length - 1] : null;
      return match ? (match[1] || '').trim().toUpperCase() : '';
    })(),
  };
}

export function buildClauseClosureRows({
  clauseProofPlan,
  specAnchors = [],
  doneMeans = [],
  inScopePaths = [],
  testPlan = '',
  canonicalContractExamples = [],
}) {
  const examplesValue = uniqueOrdered(canonicalContractExamples).join(', ') || 'NONE';
  const explicitRows = (clauseProofPlan || []).map((item) => {
    const record = parsePipeRecord(item);
    const clause = record.CLAUSE || '<missing>';
    const codeSurfaces = record.EXPECTED_CODE_SURFACES || '<missing>';
    const tests = record.EXPECTED_TESTS || '<missing>';
    return `CLAUSE: ${clause} | CODE_SURFACES: ${codeSurfaces} | TESTS: ${tests} | EXAMPLES: ${examplesValue} | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING`;
  });

  if (explicitRows.length > 0) return explicitRows;

  const fallbackCodeSurfaces = uniqueOrdered(inScopePaths).join(', ') || 'See IN_SCOPE_PATHS';
  const fallbackTests = normalizeInlineBlock(testPlan, uniqueOrdered(doneMeans).join('; ') || 'See TEST_PLAN');
  const anchorRows = (specAnchors || [])
    .filter((anchor) => anchor && String(anchor.specAnchor || '').trim())
    .map((anchor) => `CLAUSE: ${String(anchor.specAnchor || '').trim()} [LEGACY_REFINEMENT_BRIDGE] | CODE_SURFACES: ${fallbackCodeSurfaces} | TESTS: ${fallbackTests} | EXAMPLES: ${examplesValue} | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING`);
  if (anchorRows.length > 0) return anchorRows;

  const doneMeansRows = uniqueOrdered(doneMeans)
    .map((item) => `CLAUSE: DONE_MEANS: ${item} [LEGACY_REFINEMENT_BRIDGE] | CODE_SURFACES: ${fallbackCodeSurfaces} | TESTS: ${fallbackTests} | EXAMPLES: ${examplesValue} | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING`);
  return doneMeansRows;
}

export function deriveSharedSurfaceMonitoring({ hotFiles, tripwireTests, inScopePaths = [] }) {
  const normalizedHotFiles = uniqueOrdered(hotFiles);
  const normalizedInScopePaths = uniqueOrdered(inScopePaths);
  const derivedSharedHotFiles = normalizedHotFiles.length > 0
    ? normalizedHotFiles
    : normalizedInScopePaths.filter((file) => SHARED_SURFACE_PATTERNS.some((pattern) => pattern.test(file)));
  const sharedSurfaceRisk = uniqueOrdered([...normalizedHotFiles, ...normalizedInScopePaths]).some((file) => SHARED_SURFACE_PATTERNS.some((pattern) => pattern.test(file))) ? 'YES' : 'NO';
  const normalizedTripwireTests = uniqueOrdered(tripwireTests);
  const derivedTripwireTests = normalizedTripwireTests.length > 0
    ? normalizedTripwireTests
    : (sharedSurfaceRisk === 'YES' ? [normalizeInlineBlock('', 'Legacy bridge: use packet TEST_PLAN plus validator spot-check on integrated main')] : []);
  return {
    sharedSurfaceRisk,
    hotFiles: derivedSharedHotFiles,
    requiredTripwireTests: derivedTripwireTests,
    postMergeSpotcheckRequired: sharedSurfaceRisk === 'YES' ? 'YES' : 'NO',
  };
}

export function formatClauseClosureMatrixSection(clauseRows) {
  return `## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
${formatList(clauseRows)}`;
}

export function buildPacketAcceptanceRowsFromClauses(clauseRows = []) {
  return (clauseRows || []).map((row, index) => {
    const record = parsePipeRecord(row);
    const id = `AC-${String(index + 1).padStart(3, '0')}`;
    const clause = String(record.CLAUSE || `Clause row ${index + 1}`).trim();
    const tests = String(record.TESTS || '').trim();
    const examples = String(record.EXAMPLES || '').trim();
    const codeSurfaces = String(record.CODE_SURFACES || '').trim();
    const coderStatus = canonicalizeCoderStatus(record.CODER_STATUS);
    const validatorStatus = canonicalizeValidatorStatus(record.VALIDATOR_STATUS);
    const notApplicable = coderStatus === 'NOT_APPLICABLE' || validatorStatus === 'NOT_APPLICABLE';
    const status = notApplicable
      ? 'NOT_APPLICABLE'
      : (coderStatus === 'PROVED' && validatorStatus === 'CONFIRMED')
        ? 'CONFIRMED'
        : 'PENDING';
    const evidence = [tests, examples, codeSurfaces]
      .find((value) => value && !/^(NONE|N\/A|TBD|PENDING|<.*>)$/i.test(value))
      || 'CLAUSE_CLOSURE_MATRIX';
    const reason = notApplicable ? 'Inherited NOT_APPLICABLE from clause closure row' : 'NONE';
    return `ID: ${id} | REQUIREMENT: ${clause} | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: ${status} | EVIDENCE: ${evidence} | REASON: ${reason}`;
  });
}

export function formatPacketAcceptanceMatrixSection(clauseRows = []) {
  return `## PACKET_ACCEPTANCE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the executable acceptance contract for packet closure. New packets must keep stable row IDs and move each required row to PROVED, CONFIRMED, or NOT_APPLICABLE with evidence before PASS.
- Rule: use STEER or BLOCKED for unresolved required rows instead of narrative closure.
- ACCEPTANCE_ROWS:
${formatList(buildPacketAcceptanceRowsFromClauses(clauseRows))}`;
}

function canonicalizeCoderStatus(rawValue) {
  const normalized = String(rawValue || '').trim().toUpperCase();
  if (!normalized) return 'UNPROVEN';
  if (normalized === 'PROVEN') return 'PROVED';
  return normalized;
}

function canonicalizeValidatorStatus(rawValue) {
  const normalized = String(rawValue || '').trim().toUpperCase();
  if (!normalized) return 'PENDING';
  if (normalized === 'PASS') return 'CONFIRMED';
  if (normalized === 'FAIL') return 'REJECTED';
  return normalized;
}

function validatorAllowsCoderProof(validatorStatus) {
  return ['CONFIRMED', 'NOT_APPLICABLE'].includes(String(validatorStatus || '').trim().toUpperCase());
}

function normalizeAcceptanceStatus(rawValue) {
  const normalized = String(rawValue || '').trim().toUpperCase();
  if (!normalized) return 'PENDING';
  if (normalized === 'PROVEN') return 'PROVED';
  if (normalized === 'PASS') return 'CONFIRMED';
  if (normalized === 'N/A') return 'NOT_APPLICABLE';
  return normalized;
}

function hasConcreteEvidence(value) {
  const normalized = String(value || '').trim();
  return Boolean(normalized && !/^(NONE|N\/A|TBD|PENDING|<.*>)$/i.test(normalized));
}

function hasConcreteReason(value) {
  const normalized = String(value || '').trim();
  return Boolean(normalized && !/^(NONE|N\/A|TBD|PENDING|<.*>)$/i.test(normalized));
}

function buildLegacyAcceptanceRowsFromClauses(clauseRows = []) {
  return (clauseRows || []).map((row, index) => {
    const validatorConfirmed = ['CONFIRMED', 'NOT_APPLICABLE'].includes(String(row.validatorStatus || '').trim().toUpperCase());
    const coderProved = ['PROVED', 'NOT_APPLICABLE'].includes(String(row.coderStatus || '').trim().toUpperCase());
    const notApplicable = row.validatorStatus === 'NOT_APPLICABLE' || row.coderStatus === 'NOT_APPLICABLE';
    const status = notApplicable
      ? 'NOT_APPLICABLE'
      : validatorConfirmed && coderProved
        ? 'CONFIRMED'
        : 'PENDING';
    return {
      id: `LEGACY-CLAUSE-${String(index + 1).padStart(3, '0')}`,
      requirement: row.clause,
      required: 'YES',
      evidenceKind: 'CLAUSE_CLOSURE_MATRIX',
      owner: 'VALIDATOR',
      status,
      evidence: row.tests || row.examples || row.codeSurfaces || 'CLAUSE_CLOSURE_MATRIX',
      reason: notApplicable ? 'Inherited NOT_APPLICABLE from CLAUSE_CLOSURE_MATRIX' : '',
      raw: row.clause,
      legacyDerived: true,
    };
  });
}

export function validatePacketAcceptanceMatrix(packetText, {
  requirePassClosure = false,
  legacyClauseRows = null,
} = {}) {
  const errors = [];
  const section = extractSectionAfterHeading(packetText, 'PACKET_ACCEPTANCE_MATRIX');
  if (!section) {
    return {
      errors,
      parsed: {
        explicit: false,
        rows: Array.isArray(legacyClauseRows) ? buildLegacyAcceptanceRowsFromClauses(legacyClauseRows) : [],
      },
    };
  }

  const rawRows = extractListItemsAfterLabel(section, 'ACCEPTANCE_ROWS')
    .filter((item) => !/^NONE$/i.test(item || ''));
  const rows = [];
  const seenIds = new Set();
  for (const item of rawRows) {
    const record = parsePipeRecord(item);
    const id = String(record.ID || '').trim();
    const requirement = String(record.REQUIREMENT || '').trim();
    const required = String(record.REQUIRED || 'YES').trim().toUpperCase();
    const evidenceKind = String(record.EVIDENCE_KIND || '').trim().toUpperCase();
    const owner = String(record.OWNER || '').trim().toUpperCase();
    const status = normalizeAcceptanceStatus(record.STATUS);
    const evidence = String(record.EVIDENCE || '').trim();
    const reason = String(record.REASON || '').trim();

    if (!id || !requirement || !/^(YES|NO)$/.test(required) || !evidenceKind || !owner) {
      errors.push(`PACKET_ACCEPTANCE_MATRIX malformed row: ${item}`);
      continue;
    }
    if (seenIds.has(id)) {
      errors.push(`PACKET_ACCEPTANCE_MATRIX duplicate ID: ${id}`);
      continue;
    }
    seenIds.add(id);
    if (!/^(PROVED|CONFIRMED|NOT_APPLICABLE|PENDING|STEER|BLOCKED)$/.test(status)) {
      errors.push(`PACKET_ACCEPTANCE_MATRIX row has invalid STATUS=${status}: ${id}`);
      continue;
    }
    if (status === 'NOT_APPLICABLE' && !hasConcreteReason(reason)) {
      errors.push(`PACKET_ACCEPTANCE_MATRIX NOT_APPLICABLE row requires REASON: ${id}`);
      continue;
    }
    if (['PROVED', 'CONFIRMED'].includes(status) && !hasConcreteEvidence(evidence)) {
      errors.push(`PACKET_ACCEPTANCE_MATRIX ${status} row requires concrete EVIDENCE: ${id}`);
      continue;
    }

    rows.push({
      id,
      requirement,
      required,
      evidenceKind,
      owner,
      status,
      evidence,
      reason,
      raw: item,
      legacyDerived: false,
    });
  }

  if (rawRows.length === 0) {
    errors.push('PACKET_ACCEPTANCE_MATRIX must list one or more ACCEPTANCE_ROWS when the section is present');
  }

  if (requirePassClosure) {
    for (const row of rows) {
      if (row.required !== 'YES') continue;
      if (!['PROVED', 'CONFIRMED', 'NOT_APPLICABLE'].includes(row.status)) {
        errors.push(`SPEC pass closure requires required PACKET_ACCEPTANCE_MATRIX row ${row.id} to be PROVED, CONFIRMED, or NOT_APPLICABLE`);
      }
      if (['STEER', 'BLOCKED'].includes(row.status)) {
        errors.push(`SPEC pass closure cannot leave PACKET_ACCEPTANCE_MATRIX row ${row.id} at STATUS=${row.status}`);
      }
    }
  }

  return {
    errors,
    parsed: {
      explicit: true,
      rows,
    },
  };
}

export function normalizeActiveClauseClosureMatrix(packetText) {
  const clauseSection = extractSectionAfterHeading(packetText, 'CLAUSE_CLOSURE_MATRIX');
  if (!clauseSection) {
    return {
      changed: false,
      packetText: String(packetText || ''),
      repairs: [],
    };
  }

  const rawClauseRows = extractListItemsAfterLabel(clauseSection, 'CLAUSE_ROWS');
  if (rawClauseRows.length === 0) {
    return {
      changed: false,
      packetText: String(packetText || ''),
      repairs: [],
    };
  }

  const repairs = [];
  let changed = false;
  const nextClauseRows = rawClauseRows.map((item) => {
    if (/^NONE$/i.test(item || '')) return item;
    const record = parsePipeRecord(item);
    const clause = String(record.CLAUSE || '').trim();
    const codeSurfaces = String(record.CODE_SURFACES || '').trim();
    const tests = String(record.TESTS || 'NONE').trim() || 'NONE';
    const examples = String(record.EXAMPLES || 'NONE').trim() || 'NONE';
    const debtIds = String(record.DEBT_IDS || 'NONE').trim() || 'NONE';
    const originalValidatorStatus = String(record.VALIDATOR_STATUS || 'PENDING').trim().toUpperCase() || 'PENDING';
    const validatorStatus = canonicalizeValidatorStatus(originalValidatorStatus);
    const originalCoderStatus = String(record.CODER_STATUS || 'UNPROVEN').trim().toUpperCase() || 'UNPROVEN';
    let nextCoderStatus = canonicalizeCoderStatus(originalCoderStatus);

    if (originalCoderStatus === 'PROVEN') {
      changed = true;
      repairs.push(`canonicalized invalid CODER_STATUS=PROVEN to PROVED for clause ${clause || '<unknown>'}`);
    }
    if (originalValidatorStatus !== validatorStatus) {
      changed = true;
      repairs.push(`canonicalized invalid VALIDATOR_STATUS=${originalValidatorStatus} to ${validatorStatus} for clause ${clause || '<unknown>'}`);
    }

    if (nextCoderStatus === 'PROVED' && !validatorAllowsCoderProof(validatorStatus)) {
      nextCoderStatus = 'UNPROVEN';
      changed = true;
      repairs.push(`reset premature coder proof on clause ${clause || '<unknown>'} because VALIDATOR_STATUS=${validatorStatus}`);
    }

    return `CLAUSE: ${clause} | CODE_SURFACES: ${codeSurfaces} | TESTS: ${tests} | EXAMPLES: ${examples} | DEBT_IDS: ${debtIds} | CODER_STATUS: ${nextCoderStatus} | VALIDATOR_STATUS: ${validatorStatus}`;
  });

  if (!changed) {
    return {
      changed: false,
      packetText: String(packetText || ''),
      repairs,
    };
  }

  return {
    changed: true,
    packetText: replaceSection(String(packetText || ''), 'CLAUSE_CLOSURE_MATRIX', formatClauseClosureMatrixSection(nextClauseRows)),
    repairs,
  };
}

export function formatSpecDebtStatusSection({ openSpecDebt = 'NO', blockingSpecDebt = 'NO', debtIds = [] } = {}) {
  const normalizedDebtIds = (debtIds || []).map((item) => String(item || '').trim()).filter(Boolean);
  return `## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: ${openSpecDebt}
- BLOCKING_SPEC_DEBT: ${blockingSpecDebt}
- DEBT_IDS: ${normalizedDebtIds.length > 0 ? normalizedDebtIds.join(', ') : 'NONE'}
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.`;
}

export function formatSharedSurfaceMonitoringSection({
  sharedSurfaceRisk = 'NO',
  hotFiles = [],
  requiredTripwireTests = [],
  postMergeSpotcheckRequired = 'NO',
} = {}) {
  return `## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: ${sharedSurfaceRisk}
- HOT_FILES:
${formatList(hotFiles)}
- REQUIRED_TRIPWIRE_TESTS:
${formatList(requiredTripwireTests)}
- POST_MERGE_SPOTCHECK_REQUIRED: ${postMergeSpotcheckRequired}
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.`;
}

export function validatePacketClosureMonitoring(packetText, {
  requireRows = false,
  requireClosedConsistency = false,
  requirePassConsistency = false,
} = {}) {
  const errors = [];

  const clauseSection = extractSectionAfterHeading(packetText, 'CLAUSE_CLOSURE_MATRIX');
  const specDebtSection = extractSectionAfterHeading(packetText, 'SPEC_DEBT_STATUS');
  const sharedSurfaceSection = extractSectionAfterHeading(packetText, 'SHARED_SURFACE_MONITORING');

  if (!clauseSection) errors.push('CLAUSE_CLOSURE_MATRIX section missing');
  if (!specDebtSection) errors.push('SPEC_DEBT_STATUS section missing');
  if (!sharedSurfaceSection) errors.push('SHARED_SURFACE_MONITORING section missing');
  if (errors.length > 0) return { errors, parsed: null };

  const rawClauseRows = extractListItemsAfterLabel(clauseSection, 'CLAUSE_ROWS');
  const clauseRows = [];
  for (const item of rawClauseRows) {
    if (/^NONE$/i.test(item || '')) continue;
    const record = parsePipeRecord(item);
    const clause = String(record.CLAUSE || '').trim();
    const codeSurfaces = String(record.CODE_SURFACES || '').trim();
    const tests = String(record.TESTS || '').trim();
    const examples = String(record.EXAMPLES || 'NONE').trim() || 'NONE';
    const debtIdsRaw = String(record.DEBT_IDS || 'NONE').trim() || 'NONE';
    const coderStatus = String(record.CODER_STATUS || '').trim().toUpperCase();
    const validatorStatus = String(record.VALIDATOR_STATUS || '').trim().toUpperCase();
    if (
      !clause
      || !codeSurfaces
      || !tests
      || !/^(UNPROVEN|PROVED|PARTIAL|DEFERRED|NOT_APPLICABLE)$/.test(coderStatus)
      || !/^(PENDING|CONFIRMED|PARTIAL|REJECTED|NOT_APPLICABLE)$/.test(validatorStatus)
    ) {
      errors.push(`CLAUSE_CLOSURE_MATRIX malformed row: ${item}`);
      continue;
    }
    // RGF-188: reject placeholder values in core matrix fields at handoff time
    // instead of letting them through to closeout where they consume orchestrator budget.
    const placeholderRe = /^(?:\{.+\}|<fill.*>|<pending>|<unclaimed>|<paste>|tbd)$/i;
    if (placeholderRe.test(clause)) {
      errors.push(`CLAUSE_CLOSURE_MATRIX row has placeholder CLAUSE value: ${item}`);
      continue;
    }
    if (placeholderRe.test(codeSurfaces)) {
      errors.push(`CLAUSE_CLOSURE_MATRIX row has placeholder CODE_SURFACES value: ${item}`);
      continue;
    }
    if (placeholderRe.test(tests)) {
      errors.push(`CLAUSE_CLOSURE_MATRIX row has placeholder TESTS value: ${item}`);
      continue;
    }
    const rowDebtIds = /^NONE$/i.test(debtIdsRaw)
      ? []
      : debtIdsRaw.split(',').map((value) => value.trim()).filter(Boolean);
    if (rowDebtIds.some((value) => !/^SPECDEBT-[A-Za-z0-9][A-Za-z0-9_-]*$/i.test(value))) {
      errors.push(`CLAUSE_CLOSURE_MATRIX row has invalid DEBT_IDS: ${item}`);
      continue;
    }
    clauseRows.push({
      clause,
      codeSurfaces,
      tests,
      examples,
      debtIdsRaw,
      debtIds: rowDebtIds,
      coderStatus,
      validatorStatus,
    });
  }

  if (requireRows && clauseRows.length === 0) {
    errors.push('CLAUSE_CLOSURE_MATRIX must list one or more concrete CLAUSE_ROWS');
  }

  const acceptanceMatrixValidation = validatePacketAcceptanceMatrix(packetText, {
    legacyClauseRows: clauseRows,
  });
  for (const error of acceptanceMatrixValidation.errors) errors.push(error);

  const openSpecDebt = parseSingleField(specDebtSection, 'OPEN_SPEC_DEBT').toUpperCase();
  const blockingSpecDebt = parseSingleField(specDebtSection, 'BLOCKING_SPEC_DEBT').toUpperCase();
  const debtIdsRaw = parseSingleField(specDebtSection, 'DEBT_IDS');
  const debtIds = !debtIdsRaw || /^NONE$/i.test(debtIdsRaw) ? [] : debtIdsRaw.split(',').map((item) => item.trim()).filter(Boolean);

  if (!/^(YES|NO)$/.test(openSpecDebt)) errors.push('SPEC_DEBT_STATUS OPEN_SPEC_DEBT must be YES or NO');
  if (!/^(YES|NO)$/.test(blockingSpecDebt)) errors.push('SPEC_DEBT_STATUS BLOCKING_SPEC_DEBT must be YES or NO');
  if (!debtIdsRaw) errors.push('SPEC_DEBT_STATUS DEBT_IDS must be set (SPECDEBT-... or NONE)');

  const sharedSurfaceRisk = parseSingleField(sharedSurfaceSection, 'SHARED_SURFACE_RISK').toUpperCase();
  const postMergeSpotcheckRequired = parseSingleField(sharedSurfaceSection, 'POST_MERGE_SPOTCHECK_REQUIRED').toUpperCase();
  const hotFiles = extractListItemsAfterLabel(sharedSurfaceSection, 'HOT_FILES').filter((item) => !/^NONE$/i.test(item || ''));
  const requiredTripwireTests = extractListItemsAfterLabel(sharedSurfaceSection, 'REQUIRED_TRIPWIRE_TESTS').filter((item) => !/^NONE$/i.test(item || ''));

  if (!/^(YES|NO)$/.test(sharedSurfaceRisk)) errors.push('SHARED_SURFACE_MONITORING SHARED_SURFACE_RISK must be YES or NO');
  if (!/^(YES|NO)$/.test(postMergeSpotcheckRequired)) errors.push('SHARED_SURFACE_MONITORING POST_MERGE_SPOTCHECK_REQUIRED must be YES or NO');
  if (sharedSurfaceRisk === 'YES' && hotFiles.length === 0) {
    errors.push('SHARED_SURFACE_MONITORING HOT_FILES must list one or more concrete files when SHARED_SURFACE_RISK=YES');
  }
  if (sharedSurfaceRisk === 'YES' && requiredTripwireTests.length === 0) {
    errors.push('SHARED_SURFACE_MONITORING REQUIRED_TRIPWIRE_TESTS must list one or more concrete tests when SHARED_SURFACE_RISK=YES');
  }

  const hasPartialOrDeferred = clauseRows.some((row) => ['PARTIAL', 'DEFERRED'].includes(row.coderStatus) || row.validatorStatus === 'PARTIAL');
  if (hasPartialOrDeferred) {
    if (openSpecDebt !== 'YES') {
      errors.push('SPEC_DEBT_STATUS OPEN_SPEC_DEBT must be YES when any clause row is PARTIAL or DEFERRED');
    }
    if (debtIds.length === 0) {
      errors.push('SPEC_DEBT_STATUS DEBT_IDS must not be NONE when any clause row is PARTIAL or DEFERRED');
    }
  }

  const debtRegistry = loadSpecDebtRegistry();
  for (const registryError of debtRegistry.errors) errors.push(registryError);
  if (debtIds.length > 0 && debtRegistry.errors.length === 0) {
    const wpIdMatch = String(packetText || '').match(/^\s*-\s*(?:\*\*)?WP_ID(?:\*\*)?\s*:\s*(.+)\s*$/mi);
    const packetWpId = wpIdMatch ? (wpIdMatch[1] || '').trim() : '';
    for (const debtId of debtIds) {
      const row = debtRegistry.rowsById.get(debtId);
      if (!row) {
        errors.push(`SPEC_DEBT_STATUS references missing debt id: ${debtId}`);
        continue;
      }
      if (packetWpId && row.wpId !== packetWpId) {
        errors.push(`SPEC_DEBT_STATUS debt ${debtId} belongs to ${row.wpId}, expected ${packetWpId}`);
      }
      if (row.status !== 'OPEN') {
        errors.push(`SPEC_DEBT_STATUS debt ${debtId} must be STATUS=OPEN while referenced by the packet`);
      }
    }

    const registryBlockingRows = debtIds
      .map((debtId) => debtRegistry.rowsById.get(debtId))
      .filter(Boolean)
      .filter((row) => row.blocking === 'YES');
    if (blockingSpecDebt === 'YES' && registryBlockingRows.length === 0) {
      errors.push('SPEC_DEBT_STATUS BLOCKING_SPEC_DEBT=YES requires at least one referenced registry row with BLOCKING=YES');
    }
    if (blockingSpecDebt === 'NO' && registryBlockingRows.length > 0) {
      errors.push('SPEC_DEBT_STATUS BLOCKING_SPEC_DEBT=NO cannot reference registry rows marked BLOCKING=YES');
    }
  }

  for (const row of clauseRows) {
    if (row.coderStatus === 'PROVED' && !validatorAllowsCoderProof(row.validatorStatus)) {
      errors.push(`CLAUSE_CLOSURE_MATRIX row cannot use CODER_STATUS=PROVED before validator confirmation: ${row.clause}`);
    }
    if (row.debtIds.length > 0 && debtIds.length === 0) {
      errors.push(`CLAUSE_CLOSURE_MATRIX row references DEBT_IDS not reflected in SPEC_DEBT_STATUS: ${row.clause}`);
    }
    for (const debtId of row.debtIds) {
      if (!debtIds.includes(debtId)) {
        errors.push(`CLAUSE_CLOSURE_MATRIX row debt ${debtId} missing from SPEC_DEBT_STATUS DEBT_IDS`);
      }
    }
  }

  if (requireClosedConsistency) {
    if (clauseRows.some((row) => row.validatorStatus === 'PENDING')) {
      errors.push('Closed packets cannot leave any CLAUSE_CLOSURE_MATRIX row at VALIDATOR_STATUS=PENDING');
    }
  }

  if (requirePassConsistency) {
    const passAcceptanceMatrix = validatePacketAcceptanceMatrix(packetText, {
      requirePassClosure: true,
      legacyClauseRows: clauseRows,
    });
    for (const error of passAcceptanceMatrix.errors) {
      if (!errors.includes(error)) errors.push(error);
    }

    if (clauseRows.some((row) => !['PROVED', 'NOT_APPLICABLE'].includes(row.coderStatus))) {
      errors.push('SPEC pass closure requires every CLAUSE_CLOSURE_MATRIX row to use CODER_STATUS=PROVED or NOT_APPLICABLE');
    }
    if (clauseRows.some((row) => !['CONFIRMED', 'NOT_APPLICABLE'].includes(row.validatorStatus))) {
      errors.push('SPEC pass closure requires every CLAUSE_CLOSURE_MATRIX row to use VALIDATOR_STATUS=CONFIRMED or NOT_APPLICABLE');
    }
    if (openSpecDebt !== 'NO' || blockingSpecDebt !== 'NO' || debtIds.length > 0) {
      errors.push('SPEC pass closure requires SPEC_DEBT_STATUS to be OPEN_SPEC_DEBT=NO, BLOCKING_SPEC_DEBT=NO, DEBT_IDS=NONE');
    }
  }

  return {
    errors,
    parsed: {
      clauseRows,
      openSpecDebt,
      blockingSpecDebt,
      debtIds,
      debtIdsRaw,
      sharedSurfaceRisk,
      hotFiles,
      requiredTripwireTests,
      postMergeSpotcheckRequired,
      acceptanceMatrix: acceptanceMatrixValidation.parsed,
    },
  };
}

export function validateClauseReportConsistency(packetText) {
  const errors = [];
  const closureMonitorValidation = validatePacketClosureMonitoring(packetText, { requireRows: true });
  if (closureMonitorValidation.errors.length > 0) {
    return { errors: closureMonitorValidation.errors, parsed: null };
  }

  const reports = parseValidationReportsSection(packetText);
  for (const row of closureMonitorValidation.parsed.clauseRows) {
    if (row.validatorStatus === 'CONFIRMED') {
      const matched = reports.clausesReviewed.some((item) => item.includes(row.clause));
      if (!matched) {
        errors.push(`VALIDATION_REPORTS CLAUSES_REVIEWED missing clause from CLAUSE_CLOSURE_MATRIX: ${row.clause}`);
      }
    }
    if (['PARTIAL', 'REJECTED'].includes(row.validatorStatus)) {
      const matched = reports.notProven.some((item) => item.includes(row.clause));
      if (!matched) {
        errors.push(`VALIDATION_REPORTS NOT_PROVEN missing clause from CLAUSE_CLOSURE_MATRIX: ${row.clause}`);
      }
    }
  }

  return {
    errors,
    parsed: {
      ...closureMonitorValidation.parsed,
      reports,
    },
  };
}
