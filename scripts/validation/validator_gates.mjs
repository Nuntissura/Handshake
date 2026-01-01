/**
 * Validator Gates [CX-VAL-GATE]
 *
 * Mechanical enforcement of validation gate sequence.
 * Prevents auto-commit, ensures user sees report before WP append.
 *
 * Actions:
 *   present-report {WP_ID} {PASS|FAIL}  - Gate 1: Record report shown in chat
 *   acknowledge {WP_ID}                  - Gate 2: Record user acknowledgment
 *   append {WP_ID}                       - Gate 3: Record WP append completed
 *   commit {WP_ID}                       - Gate 4: Allow commit (PASS only)
 *   status {WP_ID}                       - Show current gate state
 *   reset {WP_ID}                        - Reset gates for WP (requires confirmation)
 */

import fs from 'fs';
import path from 'path';

const STATE_FILE = 'docs/VALIDATOR_GATES.json';
const MIN_GATE_INTERVAL_SECONDS = 5; // Minimum time between gates to prevent automation momentum

function loadState() {
    if (!fs.existsSync(STATE_FILE)) {
        return { validation_sessions: {} };
    }
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
}

function saveState(state) {
    fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
}

function fail(msg, details = []) {
    console.error(`[VALIDATOR GATE ERROR] ${msg}`);
    details.forEach((d) => console.error(`  - ${d}`));
    process.exit(1);
}

function success(msg, details = []) {
    console.log(`[VALIDATOR GATE] ${msg}`);
    details.forEach((d) => console.log(`  ${d}`));
}

function assertWpId(id) {
    if (!id || !id.startsWith('WP-')) {
        fail('Expected WP_ID like WP-1-Feature-Name-v1');
    }
}

function getSession(state, wpId) {
    return state.validation_sessions[wpId] || null;
}

function checkMomentum(session, gateName) {
    if (!session || !session.gates || session.gates.length === 0) return;

    const lastGate = session.gates[session.gates.length - 1];
    const lastTime = new Date(lastGate.timestamp);
    const now = new Date();
    const diffSeconds = (now.getTime() - lastTime.getTime()) / 1000;

    if (diffSeconds < MIN_GATE_INTERVAL_SECONDS) {
        fail(`Automation momentum detected for ${gateName}`, [
            `Last gate (${lastGate.gate}) was ${diffSeconds.toFixed(1)}s ago`,
            `Minimum interval: ${MIN_GATE_INTERVAL_SECONDS}s`,
            'Protocol requires user review between gates'
        ]);
    }
}

const action = process.argv[2];
const wpId = process.argv[3];
const extraArg = process.argv[4];

const state = loadState();

// =============================================================================
// ACTION: present-report {WP_ID} {PASS|FAIL}
// =============================================================================
if (action === 'present-report') {
    assertWpId(wpId);
    const verdict = extraArg?.toUpperCase();

    if (verdict !== 'PASS' && verdict !== 'FAIL') {
        fail('Verdict must be PASS or FAIL', [`Received: ${extraArg}`]);
    }

    const existing = getSession(state, wpId);
    if (existing && existing.status === 'COMMITTED') {
        fail(`${wpId} already has a committed validation session`, [
            'Create a new WP variant (e.g., WP-1-Feature-v2) for re-validation'
        ]);
    }

    // Start new session or reset if re-presenting
    state.validation_sessions[wpId] = {
        wpId,
        verdict,
        status: 'REPORT_PRESENTED',
        started: new Date().toISOString(),
        gates: [{
            gate: 'REPORT_PRESENTED',
            verdict,
            timestamp: new Date().toISOString()
        }]
    };
    saveState(state);

    success(`Gate 1 PASSED: Report presented for ${wpId}`, [
        `Verdict: ${verdict}`,
        '',
        '[HALT] Validator MUST now wait for user acknowledgment.',
        `[NEXT] After user reviews, run: just validator-gate-acknowledge ${wpId}`
    ]);
    process.exit(0);
}

// =============================================================================
// ACTION: acknowledge {WP_ID}
// =============================================================================
if (action === 'acknowledge') {
    assertWpId(wpId);

    const session = getSession(state, wpId);
    if (!session) {
        fail(`No validation session for ${wpId}`, [
            `Run: just validator-gate-present ${wpId} {PASS|FAIL}`
        ]);
    }

    if (session.status !== 'REPORT_PRESENTED') {
        fail(`Cannot acknowledge: ${wpId} is in state ${session.status}`, [
            'Expected state: REPORT_PRESENTED'
        ]);
    }

    checkMomentum(session, 'USER_ACKNOWLEDGED');

    session.status = 'USER_ACKNOWLEDGED';
    session.gates.push({
        gate: 'USER_ACKNOWLEDGED',
        timestamp: new Date().toISOString()
    });
    saveState(state);

    success(`Gate 2 PASSED: User acknowledged report for ${wpId}`, [
        '',
        '[HALT] Validator may now append report to WP.',
        `[NEXT] Run: just validator-gate-append ${wpId}`
    ]);
    process.exit(0);
}

// =============================================================================
// ACTION: append {WP_ID}
// =============================================================================
if (action === 'append') {
    assertWpId(wpId);

    const session = getSession(state, wpId);
    if (!session) {
        fail(`No validation session for ${wpId}`);
    }

    if (session.status !== 'USER_ACKNOWLEDGED') {
        fail(`Cannot append: ${wpId} is in state ${session.status}`, [
            'Expected state: USER_ACKNOWLEDGED',
            'User must acknowledge the report before it can be appended'
        ]);
    }

    checkMomentum(session, 'WP_APPENDED');

    // Verify task packet exists
    const packetPath = `docs/task_packets/${wpId}.md`;
    if (!fs.existsSync(packetPath)) {
        fail(`Task packet not found: ${packetPath}`);
    }

    session.status = 'WP_APPENDED';
    session.gates.push({
        gate: 'WP_APPENDED',
        timestamp: new Date().toISOString()
    });
    saveState(state);

    if (session.verdict === 'FAIL') {
        success(`Gate 3 PASSED: Report appended to ${wpId}`, [
            '',
            '[STOP] Verdict was FAIL - no commit allowed.',
            'WP remains open for remediation.'
        ]);
    } else {
        success(`Gate 3 PASSED: Report appended to ${wpId}`, [
            '',
            '[HALT] Validator may now commit.',
            `[NEXT] Run: just validator-gate-commit ${wpId}`
        ]);
    }
    process.exit(0);
}

// =============================================================================
// ACTION: commit {WP_ID}
// =============================================================================
if (action === 'commit') {
    assertWpId(wpId);

    const session = getSession(state, wpId);
    if (!session) {
        fail(`No validation session for ${wpId}`);
    }

    if (session.verdict !== 'PASS') {
        fail(`Cannot commit: ${wpId} verdict was ${session.verdict}`, [
            'Only PASS verdicts may be committed',
            'Fix issues and re-validate to get a PASS'
        ]);
    }

    if (session.status !== 'WP_APPENDED') {
        fail(`Cannot commit: ${wpId} is in state ${session.status}`, [
            'Expected state: WP_APPENDED',
            'Complete all prior gates before committing'
        ]);
    }

    checkMomentum(session, 'COMMITTED');

    session.status = 'COMMITTED';
    session.gates.push({
        gate: 'COMMITTED',
        timestamp: new Date().toISOString()
    });
    session.completed = new Date().toISOString();
    saveState(state);

    success(`Gate 4 PASSED: ${wpId} cleared for commit`, [
        '',
        '[UNLOCKED] Validator may now run git commit.',
        `Commit message: docs: validation PASS [${wpId}]`
    ]);
    process.exit(0);
}

// =============================================================================
// ACTION: status {WP_ID}
// =============================================================================
if (action === 'status') {
    assertWpId(wpId);

    const session = getSession(state, wpId);
    if (!session) {
        console.log(`[VALIDATOR GATE STATUS] No session for ${wpId}`);
        console.log('  Gates: (none)');
        process.exit(0);
    }

    console.log(`[VALIDATOR GATE STATUS] ${wpId}`);
    console.log(`  Verdict: ${session.verdict}`);
    console.log(`  Status: ${session.status}`);
    console.log(`  Started: ${session.started}`);
    console.log('  Gates:');
    session.gates.forEach((g, i) => {
        const check = i < session.gates.length ? '✓' : '○';
        console.log(`    ${check} ${g.gate} @ ${g.timestamp}`);
    });

    // Show next action
    const nextActions = {
        'REPORT_PRESENTED': `just validator-gate-acknowledge ${wpId}`,
        'USER_ACKNOWLEDGED': `just validator-gate-append ${wpId}`,
        'WP_APPENDED': session.verdict === 'PASS' ? `just validator-gate-commit ${wpId}` : '(FAIL - no commit)',
        'COMMITTED': '(complete)'
    };
    console.log(`  Next: ${nextActions[session.status] || 'unknown'}`);
    process.exit(0);
}

// =============================================================================
// ACTION: reset {WP_ID} --confirm
// =============================================================================
if (action === 'reset') {
    assertWpId(wpId);

    if (extraArg !== '--confirm') {
        fail('Reset requires confirmation', [
            `Run: just validator-gate-reset ${wpId} --confirm`
        ]);
    }

    const session = getSession(state, wpId);
    if (!session) {
        console.log(`[VALIDATOR GATE] No session to reset for ${wpId}`);
        process.exit(0);
    }

    // Archive old session
    if (!state.archived_sessions) state.archived_sessions = [];
    state.archived_sessions.push({
        ...session,
        archived_at: new Date().toISOString(),
        archive_reason: 'manual_reset'
    });

    delete state.validation_sessions[wpId];
    saveState(state);

    success(`Session reset for ${wpId}`, [
        'Previous session archived',
        'You may start a new validation'
    ]);
    process.exit(0);
}

// =============================================================================
// Unknown action
// =============================================================================
fail('Unknown action', [
    'Valid actions: present-report, acknowledge, append, commit, status, reset',
    '',
    'Usage:',
    '  just validator-gate-present {WP_ID} {PASS|FAIL}',
    '  just validator-gate-acknowledge {WP_ID}',
    '  just validator-gate-append {WP_ID}',
    '  just validator-gate-commit {WP_ID}',
    '  just validator-gate-status {WP_ID}',
    '  just validator-gate-reset {WP_ID} --confirm'
]);
