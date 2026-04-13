import assert from 'node:assert/strict';
import test from 'node:test';

import {
  normalizeActiveClauseClosureMatrix,
  validatePacketClosureMonitoring,
} from '../scripts/lib/packet-closure-monitor-lib.mjs';

function buildPacket(clauseRows) {
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
