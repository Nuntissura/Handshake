/**
 * Validator Gates [CX-VAL-GATE]
 *
 * Mechanical enforcement of validation gate sequence.
 * Prevents automation momentum and enforces a single review pause: the full
 * validation report is presented in chat only right before merge (PASS) or
 * remediation/discard kickoff (FAIL/ABANDONED), while still recording that the report was appended
 * to the WP packet first.
 *
 * Actions:
 *   append {WP_ID} {PASS|FAIL|ABANDONED} - Gate 1: Record WP append completed + verdict
 *   commit {WP_ID}                       - Gate 2: Clear PASS for git commit
 *   present-report {WP_ID} [PASS|FAIL|ABANDONED] - Gate 3: Record report shown in chat (blocking)
 *   acknowledge {WP_ID}                  - Gate 4: Record user acknowledgment (unlock)
 *   status {WP_ID}                       - Show current gate state
 *   reset {WP_ID}                        - Reset gates for WP (requires confirmation)
 */

import fs from 'fs';
import { spawnSync } from 'child_process';
import {
    ensureValidatorGateDir,
    resolveValidatorGatePath,
} from '../../../roles_shared/scripts/lib/validator-gate-paths.mjs';
import { GOV_ROOT_REPO_REL, workPacketPath } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';
import {
    currentGitContext,
    loadPacket,
    packetPath as resolvePacketPath,
} from '../../../roles_shared/scripts/lib/role-resume-utils.mjs';
import { REPO_ROOT } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';
import {
    evaluateValidatorPacketGovernanceState,
    evaluateValidatorPassAuthority,
    resolveValidatorActorContext,
} from '../scripts/lib/validator-governance-lib.mjs';
import {
    committedEvidenceForCloseout,
    livePrepareWorktreeHealthEvidence,
} from '../scripts/lib/committed-validation-evidence-lib.mjs';

const MIN_GATE_INTERVAL_SECONDS = 5; // Minimum time between gates to prevent automation momentum

function ensureStateDir() {
    ensureValidatorGateDir();
}

function stateFilePath(wpId) {
    return resolveValidatorGatePath(wpId);
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
    const committedEvidence = state?.committed_validation_evidence?.[wpId] || null;

    const toWrite = normalizeState({
        validation_sessions: session ? { [wpId]: session } : {},
        archived_sessions: archived,
        committed_validation_evidence: committedEvidence ? { [wpId]: committedEvidence } : {},
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

function runNode(args) {
    const result = spawnSync(process.execPath, args, {
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
    });
    return {
        code: typeof result.status === 'number' ? result.status : 1,
        output: `${result.stdout || ''}${result.stderr || ''}`.trim(),
    };
}

function assertWpId(id) {
    if (!id || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(id)) {
        fail('Expected WP_ID like WP-1-Feature-Name-v1');
    }
}

function getSession(state, wpId) {
    return state?.validation_sessions?.[wpId] || null;
}

function validatorGovernanceStateForWp(wpId, sessionStatus = '') {
    const packetContent = loadPacket(wpId);
    return evaluateValidatorPacketGovernanceState({
        wpId,
        packetPath: resolvePacketPath(wpId),
        packetContent,
        sessionStatus,
    });
}

function currentValidatorActorContextForWp(wpId) {
    const gitContext = currentGitContext();
    return resolveValidatorActorContext({
        repoRoot: gitContext.topLevel || REPO_ROOT,
        wpId,
        packetContent: loadPacket(wpId),
        gitContext,
    });
}

function failIfWrongToolLaneForGovernedGateWrite(wpId, actionName) {
    const packetContent = loadPacket(wpId);
    const workflowLane = String(packetContent.match(/^\s*-\s*WORKFLOW_LANE\s*:\s*(.+)\s*$/mi)?.[1] || '').trim().toUpperCase();
    if (workflowLane !== 'ORCHESTRATOR_MANAGED') return;

    const actorContext = currentValidatorActorContextForWp(wpId);
    if (actorContext.actorRole && actorContext.actorRole !== 'UNKNOWN') return;

    fail(`Wrong lane/tool surface for governed validator gate action ${actionName} on ${wpId}`, [
        `resolved_validator_role=${actorContext.actorRole || 'UNKNOWN'}`,
        `resolution_source=${actorContext.source || 'UNRESOLVED'}`,
        `actor_branch=${actorContext.actorBranch || '<unknown>'}`,
        `actor_worktree_dir=${actorContext.actorWorktreeDir || '<unknown>'}`,
        'Governed validator gate writes require a bound WP_VALIDATOR or INTEGRATION_VALIDATOR lane.',
        `Use: just validator-next ${wpId}`,
        `Use: just integration-validator-context-brief ${wpId}`,
        `Use: just external-validator-brief ${wpId} (read-only independent audit only)`,
    ]);
}

function ensurePassAuthorityForWp(wpId, session = null, stage = 'PASS gate') {
    const packetContent = loadPacket(wpId);
    const actorContext = currentValidatorActorContextForWp(wpId);
    const authorityCheck = evaluateValidatorPassAuthority({
        packetContent,
        actorContext,
    });
    const contextDetails = [
        `resolved_validator_role=${actorContext.actorRole || 'UNKNOWN'}`,
        `actor_branch=${actorContext.actorBranch || '<unknown>'}`,
        `actor_worktree_dir=${actorContext.actorWorktreeDir || '<unknown>'}`,
        `resolution_source=${actorContext.source || 'UNRESOLVED'}`,
    ];
    if (!authorityCheck.ok) {
        fail(`Cannot advance ${stage} for ${wpId}`, [
            ...authorityCheck.issues,
            ...contextDetails,
        ]);
    }
    if (session?.validator_role && session.validator_role !== actorContext.actorRole) {
        fail(`Cannot advance ${stage} for ${wpId}`, [
            `Existing PASS gate session belongs to ${session.validator_role}; current lane resolved to ${actorContext.actorRole}.`,
            ...contextDetails,
        ]);
    }
    if (
        session?.validator_session_key
        && actorContext.actorSessionKey
        && session.validator_session_key !== actorContext.actorSessionKey
    ) {
        fail(`Cannot advance ${stage} for ${wpId}`, [
            `Existing PASS gate session belongs to ${session.validator_session_key}; current lane resolved to ${actorContext.actorSessionKey}.`,
            ...contextDetails,
        ]);
    }
    if (
        session?.validator_session_id
        && actorContext.actorSessionId
        && session.validator_session_id !== actorContext.actorSessionId
    ) {
        fail(`Cannot advance ${stage} for ${wpId}`, [
            `Existing PASS gate session belongs to ${session.validator_session_id}; current lane resolved to ${actorContext.actorSessionId}.`,
            ...contextDetails,
        ]);
    }
    return {
        actorContext,
        authority: authorityCheck.authority,
    };
}

function failIfLegacyRemediationRequired(wpId, sessionStatus = '') {
    const governanceState = validatorGovernanceStateForWp(wpId, sessionStatus);
    if (governanceState.legacyRemediationRequired) {
        fail(`Cannot advance validator gates for ${wpId}`, [
            governanceState.message,
            `Computed policy outcome: ${governanceState.computedPolicy.outcome}`,
            `Applicability: ${governanceState.computedPolicy.applicability_reason || 'APPLICABLE'}`,
            'Request a new remediation WP variant instead of reusing this closed packet.'
        ]);
    }
    return governanceState;
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
// ACTION: present-report {WP_ID} [PASS|FAIL|ABANDONED]
// =============================================================================
if (action === 'present-report') {
    assertWpId(wpId);
    failIfWrongToolLaneForGovernedGateWrite(wpId, 'present-report');
    const state = loadWpState(wpId);
    const session = getSession(state, wpId);
    const verdictArg = extraArg?.trim() ? extraArg.trim().toUpperCase() : null;
    failIfLegacyRemediationRequired(wpId, session?.status || '');

    if (!session) {
        fail(`No validation session for ${wpId}`, [
            'Append the report to the WP packet first, then record it:',
            `Run: just validator-gate-append ${wpId} {PASS|FAIL|ABANDONED}`
        ]);
    }

    if (verdictArg && verdictArg !== 'PASS' && verdictArg !== 'FAIL' && verdictArg !== 'ABANDONED') {
        fail('Verdict must be PASS, FAIL, or ABANDONED (or omitted)', [`Received: ${extraArg}`]);
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
                `${session.verdict} flow requires append gate before final report presentation.`,
                'Expected state: WP_APPENDED',
                `Next: just validator-gate-append ${wpId} ${session.verdict}`
            ]);
        }
    }

    if (session.verdict === 'PASS') {
        ensurePassAuthorityForWp(wpId, session, 'final report presentation');
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
    failIfWrongToolLaneForGovernedGateWrite(wpId, 'acknowledge');

    const state = loadWpState(wpId);
    const session = getSession(state, wpId);
    failIfLegacyRemediationRequired(wpId, session?.status || '');
    if (!session) {
        fail(`No validation session for ${wpId}`, [
            `Run: just validator-gate-append ${wpId} {PASS|FAIL|ABANDONED}`
        ]);
    }

    if (session.status !== 'REPORT_PRESENTED') {
        fail(`Cannot acknowledge: ${wpId} is in state ${session.status}`, [
            'Expected state: REPORT_PRESENTED'
        ]);
    }

    if (session.verdict === 'PASS') {
        ensurePassAuthorityForWp(wpId, session, 'final acknowledgment');
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
    } else if (session.verdict === 'ABANDONED') {
        success(`Gate 4 PASSED: User acknowledged report for ${wpId}`, [
            '',
            '[UNLOCKED] WP may proceed to governed discard/cleanup (no merge/commit).'
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
// ACTION: append {WP_ID} {PASS|FAIL|ABANDONED}
// =============================================================================
if (action === 'append') {
    assertWpId(wpId);
    failIfWrongToolLaneForGovernedGateWrite(wpId, 'append');

    const state = loadWpState(wpId);
    const verdictArg = extraArg?.trim() ? extraArg.trim().toUpperCase() : null;

    if (verdictArg && verdictArg !== 'PASS' && verdictArg !== 'FAIL' && verdictArg !== 'ABANDONED') {
        fail('Verdict must be PASS, FAIL, or ABANDONED (or omitted when a session already exists)', [
            `Received: ${extraArg}`
        ]);
    }

    // Verify work packet exists
    const packetPath = workPacketPath(wpId);
    if (!fs.existsSync(packetPath)) {
        fail(`Work packet not found: ${packetPath}`);
    }

    let session = getSession(state, wpId);
    failIfLegacyRemediationRequired(wpId, session?.status || '');
    const nowIso = new Date().toISOString();
    if (!session) {
        if (!verdictArg) {
            fail(`Verdict required to start append gate for ${wpId}`, [
                `Run: just validator-gate-append ${wpId} {PASS|FAIL|ABANDONED}`
            ]);
        }

        let actorAuthority = null;
        if (verdictArg === 'PASS') {
            actorAuthority = ensurePassAuthorityForWp(wpId, null, 'PASS append');
        }

        session = {
            wpId,
            verdict: verdictArg,
            status: 'WP_APPENDED',
            validator_role: actorAuthority?.actorContext?.actorRole || '',
            validator_session_key: actorAuthority?.actorContext?.actorSessionKey || '',
            validator_session_id: actorAuthority?.actorContext?.actorSessionId || '',
            validator_thread_id: actorAuthority?.actorContext?.actorThreadId || '',
            validator_branch: actorAuthority?.actorContext?.actorBranch || '',
            validator_worktree_dir: actorAuthority?.actorContext?.actorWorktreeDir || '',
            technical_authority: actorAuthority?.authority?.technicalAuthority || '',
            merge_authority: actorAuthority?.authority?.mergeAuthority || '',
            started: nowIso,
            gates: [{
                gate: 'WP_APPENDED',
                verdict: verdictArg,
                timestamp: nowIso
            }]
        };
        state.validation_sessions[wpId] = session;
        saveWpState(wpId, state);

        if (session.verdict === 'FAIL' || session.verdict === 'ABANDONED') {
            success(`Gate 1 PASSED: Report appended to ${wpId}`, [
                '',
                `[NEXT] Paste the full validation report to chat now (before ${session.verdict === 'ABANDONED' ? 'discard/cleanup' : 'remediation'}), then record it:`,
                `[NEXT] Run: just validator-gate-present ${wpId}`
            ]);
        } else {
            success(`Gate 1 PASSED: Report appended to ${wpId}`, [
                '',
                '[NEXT] Record committed handoff validation against the PREPARE worktree source of truth:',
                `[NEXT] Run: just validator-handoff-check ${wpId}`,
                '',
                '[NEXT] Preflight the integration-validator final lane before PASS commit clearance:',
                `[NEXT] Run: just integration-validator-closeout-check ${wpId}`,
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
    failIfWrongToolLaneForGovernedGateWrite(wpId, 'commit');

    const state = loadWpState(wpId);
    const session = getSession(state, wpId);
    failIfLegacyRemediationRequired(wpId, session?.status || '');
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

    const actorAuthority = ensurePassAuthorityForWp(wpId, session, 'PASS commit clearance');
    session.validator_role = session.validator_role || actorAuthority.actorContext.actorRole || '';
    session.validator_session_key = session.validator_session_key || actorAuthority.actorContext.actorSessionKey || '';
    session.validator_session_id = session.validator_session_id || actorAuthority.actorContext.actorSessionId || '';
    session.validator_thread_id = session.validator_thread_id || actorAuthority.actorContext.actorThreadId || '';
    session.validator_branch = session.validator_branch || actorAuthority.actorContext.actorBranch || '';
    session.validator_worktree_dir = session.validator_worktree_dir || actorAuthority.actorContext.actorWorktreeDir || '';
    session.technical_authority = session.technical_authority || actorAuthority.authority.technicalAuthority || '';
    session.merge_authority = session.merge_authority || actorAuthority.authority.mergeAuthority || '';

    const committedEvidence = state?.committed_validation_evidence?.[wpId] || null;
    const durableCommittedProof = committedEvidenceForCloseout(committedEvidence);
    const livePrepareHealth = livePrepareWorktreeHealthEvidence(committedEvidence);
    if (!durableCommittedProof || durableCommittedProof.status !== 'PASS') {
        fail(`Cannot commit: ${wpId} is missing committed handoff validation evidence`, [
            'PASS commit clearance now requires committed validation against the PREPARE worktree source of truth.',
            `Run: just validator-handoff-check ${wpId}`,
            durableCommittedProof
                ? `Latest durable committed proof status: ${durableCommittedProof.status}`
                : 'No committed validation evidence is recorded for this WP.'
        ]);
    }

    const communicationHealth = runNode([
        `${GOV_ROOT_REPO_REL}/roles_shared/checks/wp-communication-health-check.mjs`,
        wpId,
        'VERDICT',
    ]);
    if (communicationHealth.code !== 0) {
        fail(`Cannot commit: ${wpId} is missing verdict-ready direct review communication evidence`, [
            ...communicationHealth.output.split(/\r?\n/).filter(Boolean),
        ]);
    }

    const closeoutPreflight = runNode([
        `${GOV_ROOT_REPO_REL}/roles/validator/checks/integration-validator-closeout-check.mjs`,
        wpId,
    ]);
    if (closeoutPreflight.code !== 0) {
        fail(`Cannot commit: ${wpId} failed integration-validator closeout preflight`, [
            ...closeoutPreflight.output.split(/\r?\n/).filter(Boolean),
        ]);
    }

    const computedPolicy = runNode([
        `${GOV_ROOT_REPO_REL}/roles_shared/checks/computed-policy-gate-check.mjs`,
        wpId,
    ]);
    if (computedPolicy.code !== 0) {
        fail(`Cannot commit: ${wpId} failed the computed policy gate`, [
            ...computedPolicy.output.split(/\r?\n/).filter(Boolean),
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
        ...(livePrepareHealth && livePrepareHealth.status !== 'PASS'
            ? [`Live PREPARE worktree health remains ${livePrepareHealth.status}; commit clearance is using the durable committed target proof for ${durableCommittedProof.target_head_sha}.`]
            : []),
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
    const governanceState = validatorGovernanceStateForWp(wpId, session?.status || '');
    if (!session) {
        console.log(`[VALIDATOR GATE STATUS] No session for ${wpId}`);
        if (governanceState.legacyRemediationRequired) {
            console.log(`  Governance outcome: ${governanceState.computedPolicy.outcome}`);
            console.log(`  Governance applicability: ${governanceState.computedPolicy.applicability_reason || 'APPLICABLE'}`);
            console.log('  Governance block: legacy remediation required');
            console.log('  Next: BLOCKED - request new remediation WP variant; do not merge or reopen this packet in-place');
            process.exit(0);
        }
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
    if (governanceState.legacyRemediationRequired) {
        console.log(`  Governance outcome: ${governanceState.computedPolicy.outcome}`);
        console.log(`  Governance applicability: ${governanceState.computedPolicy.applicability_reason || 'APPLICABLE'}`);
        console.log('  Governance block: legacy remediation required');
    }
    if (session.validator_role || session.validator_session_id || session.validator_session_key) {
        console.log('  Validator lane:');
        console.log(`    Role: ${session.validator_role || '<unknown>'}`);
        console.log(`    Session key: ${session.validator_session_key || '<none>'}`);
        console.log(`    Session id: ${session.validator_session_id || '<none>'}`);
        console.log(`    Thread id: ${session.validator_thread_id || '<none>'}`);
        console.log(`    Branch: ${session.validator_branch || '<unknown>'}`);
        console.log(`    Worktree: ${session.validator_worktree_dir || '<unknown>'}`);
        console.log(`    Technical authority: ${session.technical_authority || '<unspecified>'}`);
        console.log(`    Merge authority: ${session.merge_authority || '<unspecified>'}`);
    }
    const committedEvidence = state?.committed_validation_evidence?.[wpId] || null;
    const durableCommittedProof = committedEvidenceForCloseout(committedEvidence);
    const livePrepareHealth = livePrepareWorktreeHealthEvidence(committedEvidence);
    if (durableCommittedProof || livePrepareHealth) {
        console.log('  Committed validation:');
        console.log(`    Durable proof status: ${durableCommittedProof?.status || 'NONE'}`);
        console.log(`    Durable target: ${durableCommittedProof?.committed_validation_target || '<none>'}`);
        console.log(`    Durable HEAD: ${durableCommittedProof?.target_head_sha || '<none>'}`);
        console.log(`    Durable validated at: ${durableCommittedProof?.validated_at || '<none>'}`);
        console.log(`    Live PREPARE health: ${livePrepareHealth?.status || 'NONE'}`);
        console.log(`    Worktree: ${livePrepareHealth?.prepare_worktree_dir || durableCommittedProof?.prepare_worktree_dir || '<none>'}`);
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
                committedEvidenceForCloseout(committedEvidence)?.status === 'PASS'
                    ? `just validator-gate-commit ${wpId}`
                    : `just validator-handoff-check ${wpId}`
            )
            : `just validator-gate-present ${wpId}`,
        'COMMITTED': `just validator-gate-present ${wpId}`,
        'REPORT_PRESENTED': `just validator-gate-acknowledge ${wpId}`,
        'USER_ACKNOWLEDGED': session.verdict === 'PASS'
            ? '(PASS - merge/push allowed)'
            : session.verdict === 'ABANDONED'
                ? '(ABANDONED - discard/cleanup allowed)'
                : '(FAIL - remediation allowed)'
    };
    console.log(`  Next: ${governanceState.legacyRemediationRequired
        ? 'BLOCKED - request new remediation WP variant; do not merge or reopen this packet in-place'
        : (nextActions[session.status] || 'unknown')}`);
    process.exit(0);
}

// =============================================================================
// ACTION: reset {WP_ID} --confirm
// =============================================================================
if (action === 'reset') {
    assertWpId(wpId);
    failIfWrongToolLaneForGovernedGateWrite(wpId, 'reset');

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
    '  just validator-gate-append {WP_ID} {PASS|FAIL|ABANDONED}',
    '  just validator-gate-commit {WP_ID}',
    '  just validator-gate-present {WP_ID} [PASS|FAIL|ABANDONED]',
    '  just validator-gate-acknowledge {WP_ID}',
    '  just validator-gate-status {WP_ID}',
    '  just validator-gate-reset {WP_ID} --confirm'
]);
