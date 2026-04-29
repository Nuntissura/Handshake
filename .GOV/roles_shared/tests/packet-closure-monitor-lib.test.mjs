import assert from 'node:assert/strict';
import test from 'node:test';

import {
  normalizeActiveClauseClosureMatrix,
  validatePacketAcceptanceMatrix,
  validatePacketClosureMonitoring,
} from '../scripts/lib/packet-closure-monitor-lib.mjs';

function buildPacket(clauseRows, { acceptanceRows = null } = {}) {
  return [
    '# Task Packet: WP-TEST-CLOSURE-MONITOR-v1',
    '',
    '**Status:** In Progress',
    '',
    '## METADATA',
    '- WP_ID: WP-TEST-CLOSURE-MONITOR-v1',
    '- PACKET_FORMAT_VERSION: 2026-04-06',
    '- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1',
    '',
    '## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)',
    '- Rule: this is the live packet-scope monitor for diff-scoped spec closure.',
    '- CLAUSE_ROWS:',
    ...clauseRows.map((row) => `  - ${row}`),
    '',
    '## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)',
    '- OPEN_SPEC_DEBT: NO',
    '- BLOCKING_SPEC_DEBT: NO',
    '- DEBT_IDS: NONE',
    '',
    '## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)',
    '- SHARED_SURFACE_RISK: NO',
    '- HOT_FILES:',
    '  - NONE',
    '- REQUIRED_TRIPWIRE_TESTS:',
    '  - NONE',
    '- POST_MERGE_SPOTCHECK_REQUIRED: NO',
    '',
    ...(acceptanceRows
      ? [
          '## PACKET_ACCEPTANCE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)',
          '- ACCEPTANCE_ROWS:',
          ...acceptanceRows.map((row) => `  - ${row}`),
          '',
        ]
      : []),
  ].join('\n');
}

test('active clause rows cannot claim coder proof before validator confirmation', () => {
  const packetText = buildPacket([
    'CLAUSE: Demo clause | CODE_SURFACES: src/demo.rs | TESTS: cargo test demo | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: PENDING',
  ]);

  const result = validatePacketClosureMonitoring(packetText, { requireRows: true });
  assert.match(result.errors.join('\n'), /cannot use CODER_STATUS=PROVED before validator confirmation/i);
});

test('active clause normalization heals invalid PROVEN and premature proof states', () => {
  const packetText = buildPacket([
    'CLAUSE: Demo clause | CODE_SURFACES: src/demo.rs | TESTS: cargo test demo | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVEN | VALIDATOR_STATUS: PENDING',
    'CLAUSE: Confirmed clause | CODE_SURFACES: src/ok.rs | TESTS: cargo test ok | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVEN | VALIDATOR_STATUS: CONFIRMED',
  ]);

  const normalized = normalizeActiveClauseClosureMatrix(packetText);
  assert.equal(normalized.changed, true);
  assert.match(normalized.packetText, /Demo clause .*CODER_STATUS: UNPROVEN \| VALIDATOR_STATUS: PENDING/i);
  assert.match(normalized.packetText, /Confirmed clause .*CODER_STATUS: PROVED \| VALIDATOR_STATUS: CONFIRMED/i);
});

test('active clause normalization canonicalizes legacy validator PASS verdicts', () => {
  const packetText = buildPacket([
    'CLAUSE: Legacy clause | CODE_SURFACES: src/legacy.rs | TESTS: cargo test legacy | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PASS',
  ]);

  const normalized = normalizeActiveClauseClosureMatrix(packetText);
  assert.equal(normalized.changed, true);
  assert.match(normalized.packetText, /Legacy clause .*CODER_STATUS: UNPROVEN \| VALIDATOR_STATUS: CONFIRMED/i);
});

test('explicit packet acceptance matrix blocks PASS closure with steered required rows', () => {
  const packetText = buildPacket([
    'CLAUSE: Demo clause | CODE_SURFACES: src/demo.rs | TESTS: cargo test demo | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED',
  ], {
    acceptanceRows: [
      'ID: AC-001 | REQUIREMENT: Run focused test | REQUIRED: YES | EVIDENCE_KIND: COMMAND | OWNER: CODER | STATUS: STEER | EVIDENCE: NONE | REASON: waiting for coder',
    ],
  });

  const result = validatePacketClosureMonitoring(packetText, { requireRows: true, requirePassConsistency: true });
  assert.match(result.errors.join('\n'), /PACKET_ACCEPTANCE_MATRIX row AC-001/i);
});

test('explicit packet acceptance matrix requires evidence for proved rows and reason for not applicable rows', () => {
  const packetText = buildPacket([
    'CLAUSE: Demo clause | CODE_SURFACES: src/demo.rs | TESTS: cargo test demo | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING',
  ], {
    acceptanceRows: [
      'ID: AC-001 | REQUIREMENT: Run focused test | REQUIRED: YES | EVIDENCE_KIND: COMMAND | OWNER: CODER | STATUS: PROVED | EVIDENCE: NONE | REASON: NONE',
      'ID: AC-002 | REQUIREMENT: Browser smoke | REQUIRED: YES | EVIDENCE_KIND: COMMAND | OWNER: VALIDATOR | STATUS: NOT_APPLICABLE | EVIDENCE: NONE | REASON: NONE',
    ],
  });

  const result = validatePacketAcceptanceMatrix(packetText);
  assert.match(result.errors.join('\n'), /PROVED row requires concrete EVIDENCE: AC-001/i);
  assert.match(result.errors.join('\n'), /NOT_APPLICABLE row requires REASON: AC-002/i);
});

test('packets without explicit acceptance matrix expose legacy clause-derived rows', () => {
  const packetText = buildPacket([
    'CLAUSE: Confirmed clause | CODE_SURFACES: src/ok.rs | TESTS: cargo test ok | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED',
  ]);

  const result = validatePacketClosureMonitoring(packetText, { requireRows: true, requirePassConsistency: true });
  assert.deepEqual(result.errors, []);
  assert.equal(result.parsed.acceptanceMatrix.explicit, false);
  assert.equal(result.parsed.acceptanceMatrix.rows[0].id, 'LEGACY-CLAUSE-001');
  assert.equal(result.parsed.acceptanceMatrix.rows[0].status, 'CONFIRMED');
});
