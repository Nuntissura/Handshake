/**
 * Validator Gates [CX-VAL-GATE]
 *
 * Mechanical enforcement of validation gate sequence.
 * Prevents automation momentum and enforces a single review pause: the full
 * validation report is presented in chat only right before merge (PASS) or
 * remediation kickoff (FAIL), while still recording that the report was appended
 * to the WP packet first.
 *
 * Actions:
 *   append {WP_ID} {PASS|FAIL}           - Gate 1: Record WP append completed + verdict
 *   commit {WP_ID}                       - Gate 2: Clear PASS for git commit
 *   present-report {WP_ID} [PASS|FAIL]   - Gate 3: Record report shown in chat (blocking)
 *   acknowledge {WP_ID}                  - Gate 4: Record user acknowledgment (unlock)
 *   status {WP_ID}                       - Show current gate state
 *   reset {WP_ID}                        - Reset gates for WP (requires confirmation)
 */

import fs from 'fs';
import {
    ensureValidatorGateDir,
    validatorGatePath,
    resolveValidatorGatePath,
} from '../../../roles_shared/scripts/lib/validator-gate-paths.mjs';

const MIN_GATE_INTERVAL_SECONDS = 5; // Minimum time between gates to prevent automation momentum

function ensureStateDir() {
    ensureValidatorGateDir();
}

function stateFilePath(wpId) {
    return validatorGatePath(wpId);
}

function normalizeState(raw) {
    const validation_sessions =
        raw?.validation_sessions && typeof raw.validation_sessions === 'object'
            ? raw.validation_sessions
            : {};
    const committed_validation_evidence =
        raw?.committed_validation_evidence && typeof raw.committed_validation_evidence === 'object'
            ? raw.committed_validation_evidence
            : {};

    return {
        validation_sessions,
        archived_sessions: Array.isArray(raw?.archived_sessions) ? raw.archived_sessions : [],
        committed_validation_evidence,
    };
}

function loadWpState(wpId) {
    ensureStateDir();

    const perFile = resolveValidatorGatePath(wpId);
    if (fs.existsSync(perFile)) {
        const raw = JSON.parse(fs.readFileSync(perFile, 'utf8'));
        return normalizeState(raw);
    }

    return normalizeState({});
}

function saveWpState(wpId, state) {
    ensureStateDir();
    const perFile = stateFilePath(wpId);

    const session = state?.validation_sessions?.[wpId] || null;
    const archived = Array.isArray(state?.archived_sessions)
        ? state.archived_sessions.filter((s) => s?.wpId === wpId)
        : [];

    const toWrite = normalizeState({
        validation_sessions: session ? { [wpId]: session } : {},
        archived_sessions: archived,
    });

    fs.writeFileSync(perFile, `${JSON.stringify(toWrite, null, 2)}\n`);
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
    if (!id || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(id)) {
        fail('Expected WP_ID like WP-1-Feature-Name-v1');
    }
}

function getSession(state, wpId) {
    return state?.validation_sessions?.[wpId] || null;
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

// =============================================================================
// ACTION: present-report {WP_ID} [PASS|FAIL]
// =============================================================================
if (action === 'present-report') {
    assertWpId(wpId);
    const state = loadWpState(wpId);
    const session = getSession(state, wpId);
    const verdictArg = extraArg?.trim() ? extraArg.trim().toUpperCase() : null;

    if (!session) {
        fail(`No validation session for ${wpId}`, [
            'Append the report to the WP packet first, then record it:',
            `Run: just validator-gate-append ${wpId} {PASS|FAIL}`
        ]);
    }

    if (verdictArg && verdictArg !== 'PASS' && verdictArg !== 'FAIL') {
        fail('Verdict must be PASS or FAIL (or omitted)', [`Received: ${extraArg}`]);
    }

    if (verdictArg && verdictArg !== session.verdict) {
        fail(`Verdict mismatch for ${wpId}`, [
            `Session verdict: ${session.verdict}`,
            `Provided: ${verdictArg}`
        ]);
    }

    // Enforce "present only at the end" pause:
    // - FAIL: after append
    // - PASS: after commit gate
    if (session.status === 'REPORT_PRESENTED') {
        success(`Gate 3 SKIPPED: ${wpId} already in state REPORT_PRESENTED`, [
            `Verdict: ${session.verdict}`,
            '',
            '[HALT] Await user acknowledgment.',
            `[NEXT] After user reviews, run: just validator-gate-acknowledge ${wpId}`
        ]);
        process.exit(0);
    }

    if (session.status === 'USER_ACKNOWLEDGED') {
        success(`Gate 3 SKIPPED: ${wpId} already acknowledged`, [
            `Verdict: ${session.verdict}`,
        ]);
        process.exit(0);
    }

    if (session.verdict === 'PASS') {
        if (session.status !== 'COMMITTED') {
            fail(`Cannot present report for ${wpId} in state ${session.status}`, [
                'PASS flow requires commit gate before final report presentation.',
                'Expected state: COMMITTED',
                `Next: just validator-gate-commit ${wpId}`
            ]);
        }
    } else {
        if (session.status !== 'WP_APPENDED') {
            fail(`Cannot present report for ${wpId} in state ${session.status}`, [
                'FAIL flow requires append gate before final report presentation.',
                'Expected state: WP_APPENDED',
                `Next: just validator-gate-append ${wpId} FAIL`
            ]);
        }
    }

    checkMomentum(session, 'REPORT_PRESENTED');

    session.status = 'REPORT_PRESENTED';
    session.gates.push({
        gate: 'REPORT_PRESENTED',
        verdict: session.verdict,
        timestamp: new Date().toISOString()
    });
    saveWpState(wpId, state);

    success(`Gate 3 PASSED: Report presented for ${wpId}`, [
        `Verdict: ${session.verdict}`,
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

    const state = loadWpState(wpId);
    const session = getSession(state, wpId);
    if (!session) {
        fail(`No validation session for ${wpId}`, [
            `Run: just validator-gate-append ${wpId} {PASS|FAIL}`
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
    session.completed = new Date().toISOString();
    saveWpState(wpId, state);

    if (session.verdict === 'PASS') {
        success(`Gate 4 PASSED: User acknowledged report for ${wpId}`, [
            '',
            '[UNLOCKED] Validator may now merge/push the WP to main.',
            'Ensure the validation report append is committed on the WP branch before merging.'
        ]);
    } else {
        success(`Gate 4 PASSED: User acknowledged report for ${wpId}`, [
            '',
            '[UNLOCKED] WP may proceed to remediation (no merge/commit).'
        ]);
    }
    process.exit(0);
}

// =============================================================================
// ACTION: append {WP_ID} {PASS|FAIL}
// =============================================================================
if (action === 'append') {
    assertWpId(wpId);

    const state = loadWpState(wpId);
    const verdictArg = extraArg?.trim() ? extraArg.trim().toUpperCase() : null;

    if (verdictArg && verdictArg !== 'PASS' && verdictArg !== 'FAIL') {
        fail('Verdict must be PASS or FAIL (or omitted when a session already exists)', [
            `Received: ${extraArg}`
        ]);
    }

    // Verify task packet exists
    const packetPath = `.GOV/task_packets/${wpId}.md`;
    if (!fs.existsSync(packetPath)) {
        fail(`Task packet not found: ${packetPath}`);
    }

    let session = getSession(state, wpId);
    const nowIso = new Date().toISOString();
    if (!session) {
        if (!verdictArg) {
            fail(`Verdict required to start append gate for ${wpId}`, [
                `Run: just validator-gate-append ${wpId} {PASS|FAIL}`
            ]);
        }

        session = {
            wpId,
            verdict: verdictArg,
            status: 'WP_APPENDED',
            started: nowIso,
            gates: [{
                gate: 'WP_APPENDED',
                verdict: verdictArg,
                timestamp: nowIso
            }]
        };
        state.validation_sessions[wpId] = session;
        saveWpState(wpId, state);

        if (session.verdict === 'FAIL') {
            success(`Gate 1 PASSED: Report appended to ${wpId}`, [
                '',
                '[NEXT] Paste the full validation report to chat now (before remediation), then record it:',
                `[NEXT] Run: just validator-gate-present ${wpId}`
            ]);
        } else {
            success(`Gate 1 PASSED: Report appended to ${wpId}`, [
                '',
                '[NEXT] Record committed handoff validation against the PREPARE worktree source of truth:',
                `[NEXT] Run: just validator-handoff-check ${wpId}`,
                '',
                '[NEXT] After the committed handoff check passes, record PASS commit clearance:',
                `[NEXT] Run: just validator-gate-commit ${wpId}`
            ]);
        }
        process.exit(0);
    }

    if (verdictArg && verdictArg !== session.verdict) {
        fail(`Verdict mismatch for ${wpId}`, [
            `Session verdict: ${session.verdict}`,
            `Provided: ${verdictArg}`
        ]);
    }

    if (session.status === 'WP_APPENDED') {
        success(`Gate 1 SKIPPED: ${wpId} already in state WP_APPENDED`, [
            `Verdict: ${session.verdict}`
        ]);
        process.exit(0);
    }

    fail(`Cannot append: ${wpId} is in state ${session.status}`, [
        'Append is the first gate in the sequence.',
        'If you need to re-run gates for this WP, reset the session first:',
        `Run: just validator-gate-reset ${wpId} --confirm`
    ]);
    process.exit(0);
}

// =============================================================================
// ACTION: commit {WP_ID}
// =============================================================================
if (action === 'commit') {
    assertWpId(wpId);

    const state = loadWpState(wpId);
    const session = getSession(state, wpId);
    if (!session) {
        fail(`No validation session for ${wpId}`, [
            `Run: just validator-gate-append ${wpId} PASS`
        ]);
    }

    if (session.verdict !== 'PASS') {
        fail(`Cannot commit: ${wpId} verdict was ${session.verdict}`, [
            'Only PASS verdicts may be committed',
            'Fix issues and re-validate to get a PASS'
        ]);
    }

    if (session.status === 'COMMITTED') {
        success(`Gate 2 SKIPPED: ${wpId} already in state COMMITTED`, [
            '',
            '[NEXT] Paste the full validation report to chat (right before merge), then record it:',
            `[NEXT] Run: just validator-gate-present ${wpId}`
        ]);
        process.exit(0);
    }

    if (session.status !== 'WP_APPENDED') {
        fail(`Cannot commit: ${wpId} is in state ${session.status}`, [
            'Expected state: WP_APPENDED',
            'Complete all prior gates before committing'
        ]);
    }

    const committedEvidence = state?.committed_validation_evidence?.[wpId] || null;
    if (!committedEvidence || committedEvidence.status !== 'PASS') {
        fail(`Cannot commit: ${wpId} is missing committed handoff validation evidence`, [
            'PASS commit clearance now requires committed validation against the PREPARE worktree source of truth.',
            `Run: just validator-handoff-check ${wpId}`,
            committedEvidence
                ? `Latest committed evidence status: ${committedEvidence.status}`
                : 'No committed validation evidence is recorded for this WP.'
        ]);
    }

    checkMomentum(session, 'COMMITTED');

    session.status = 'COMMITTED';
    session.gates.push({
        gate: 'COMMITTED',
        timestamp: new Date().toISOString()
    });
    saveWpState(wpId, state);

    success(`Gate 2 PASSED: ${wpId} cleared for commit`, [
        '',
        '[UNLOCKED] Validator may now run git commit.',
        `Commit message: docs: validation PASS [${wpId}]`,
        '',
        '[NEXT] After git commit, paste the full validation report to chat (right before merge), then record it:',
        `[NEXT] Run: just validator-gate-present ${wpId}`
    ]);
    process.exit(0);
}

// =============================================================================
// ACTION: status {WP_ID}
// =============================================================================
if (action === 'status') {
    assertWpId(wpId);

    const state = loadWpState(wpId);
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
    if (session.completed) {
        console.log(`  Completed: ${session.completed}`);
    }
    const committedEvidence = state?.committed_validation_evidence?.[wpId] || null;
    if (committedEvidence) {
        console.log('  Committed validation:');
        console.log(`    Status: ${committedEvidence.status}`);
        console.log(`    Target: ${committedEvidence.committed_validation_target}`);
        console.log(`    HEAD: ${committedEvidence.target_head_sha}`);
        console.log(`    Worktree: ${committedEvidence.prepare_worktree_dir}`);
        console.log(`    Validated at: ${committedEvidence.validated_at}`);
    } else {
        console.log('  Committed validation: (missing)');
    }
    console.log('  Gates:');
    session.gates.forEach((g, i) => {
        const check = i < session.gates.length ? '[x]' : '[ ]';
        console.log(`    ${check} ${g.gate} @ ${g.timestamp}`);
    });

    // Show next action
    const nextActions = {
        'WP_APPENDED': session.verdict === 'PASS'
            ? (
                committedEvidence?.status === 'PASS'
                    ? `just validator-gate-commit ${wpId}`
                    : `just validator-handoff-check ${wpId}`
            )
            : `just validator-gate-present ${wpId}`,
        'COMMITTED': `just validator-gate-present ${wpId}`,
        'REPORT_PRESENTED': `just validator-gate-acknowledge ${wpId}`,
        'USER_ACKNOWLEDGED': session.verdict === 'PASS'
            ? '(PASS - merge/push allowed)'
            : '(FAIL - remediation allowed)'
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

    const state = loadWpState(wpId);
    const session = getSession(state, wpId);
    if (!session) {
        console.log(`[VALIDATOR GATE] No session to reset for ${wpId}`);
        process.exit(0);
    }

    // Archive old session
    state.archived_sessions.push({
        ...session,
        archived_at: new Date().toISOString(),
        archive_reason: 'manual_reset'
    });

    delete state.validation_sessions[wpId];
    saveWpState(wpId, state);

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
    '  just validator-gate-append {WP_ID} {PASS|FAIL}',
    '  just validator-gate-commit {WP_ID}',
    '  just validator-gate-present {WP_ID} [PASS|FAIL]',
    '  just validator-gate-acknowledge {WP_ID}',
    '  just validator-gate-status {WP_ID}',
    '  just validator-gate-reset {WP_ID} --confirm'
]);
