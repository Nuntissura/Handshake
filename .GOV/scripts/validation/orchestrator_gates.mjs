import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import {
    defaultRefinementPath,
    resolveSpecCurrent,
    validateRefinementFile,
} from './refinement-check.mjs';

const STATE_FILE = '.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json';

function loadState() {
    if (!fs.existsSync(STATE_FILE)) {
        return { gate_logs: [] };
    }
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
}

function saveState(state) {
    fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
}

const action = process.argv[2];
const wpId = process.argv[3];
const argvData = process.argv.slice(4);
const data = argvData[0];

const state = loadState();

// === V2: Protocol-locked refinement gate (unskippable) ===
// NOTE: We keep the legacy logic below for compatibility, but V2 exits before it can run.

const SIGNATURE_AUDIT_PATH = path.join('.GOV', 'roles_shared', 'SIGNATURE_AUDIT.md');

function v2Fail(msg, details = []) {
    console.error(`[GATE ERROR] ${msg}`);
    details.forEach((d) => console.error(`- ${d}`));
    process.exit(1);
}

function v2PrintGateBlocks({ wpId, stage, next, operatorAction, gateRan, result, why, gateOutputLines, nextCommands }) {
    console.log('LIFECYCLE [CX-LIFE-001]');
    console.log(`- WP_ID: ${wpId}`);
    console.log(`- STAGE: ${stage}`);
    console.log(`- NEXT: ${next}`);
    console.log('');
    console.log(`OPERATOR_ACTION: ${operatorAction || 'NONE'}`);
    console.log('');
    console.log('GATE_OUTPUT [CX-GATE-UX-001]');
    for (const line of gateOutputLines || []) console.log(line);
    console.log('');
    console.log('GATE_STATUS [CX-GATE-UX-001]');
    console.log(`- PHASE: ${stage}`);
    console.log(`- GATE_RAN: ${gateRan}`);
    console.log(`- RESULT: ${result}`);
    console.log(`- WHY: ${why}`);
    console.log('');
    console.log('NEXT_COMMANDS [CX-GATE-UX-001]');
    for (const cmd of nextCommands || []) console.log(`- ${cmd}`);
}

function v2AssertWpId(id) {
    if (!id || !id.startsWith('WP-')) {
        v2Fail('Expected WP_ID like WP-1-Storage-Abstraction-Layer-v3');
    }
}

function v2GetSingleField(content, label) {
    const re = new RegExp(`^\\s*-\\s*${label}\\s*:\\s*(.+)\\s*$`, 'mi');
    const m = content.match(re);
    return m ? m[1].trim() : '';
}

function v2GitGrepOrEmpty(needle) {
    try {
        return execSync(`git grep -n \"${needle}\" -- .`, { encoding: 'utf8' }).trim();
    } catch {
        return '';
    }
}

function v2InsertSignatureAuditRow(auditContent, rowLine) {
    const lines = auditContent.split('\n');
    const headerIdx = lines.findIndex((l) => /^\|\s*Signature\s*\|\s*Used By\s*\|/i.test(l));
    if (headerIdx === -1) return null;

    const sepIdxRel = lines.slice(headerIdx + 1).findIndex((l) => /^\|\s*-{3,}\s*\|/.test(l));
    if (sepIdxRel === -1) return null;

    const insertAt = headerIdx + 2; // after separator line
    lines.splice(insertAt, 0, rowLine.trimEnd());
    return lines.join('\n');
}

function v2ResolveLastRefinement() {
    const logs = state.gate_logs.filter((l) => l.wpId === wpId);
    return [...logs].reverse().find((l) => l.type === 'REFINEMENT') || null;
}

function v2ResolveLastSignature() {
    const logs = state.gate_logs.filter((l) => l.wpId === wpId);
    return [...logs].reverse().find((l) => l.type === 'SIGNATURE') || null;
}

function v2ResolveLastPrepare() {
    const logs = state.gate_logs.filter((l) => l.wpId === wpId);
    return [...logs].reverse().find((l) => l.type === 'PREPARE') || null;
}

function v2NormalizeBranch(branch) {
    if (!branch) return '';
    return branch.replace(/^refs\/heads\//, '').trim();
}

function v2WorktreeListPorcelain() {
    try {
        return execSync('git worktree list --porcelain', { encoding: 'utf8' });
    } catch (e) {
        v2Fail('Failed to read git worktree list (is this a git repo?)', [String(e?.message || e)]);
    }
}

function v2WorktreeHasBranch(branch) {
    const needle = `branch refs/heads/${branch}`;
    const out = v2WorktreeListPorcelain();
    return out.split(/\r?\n/).some((line) => line.trim() === needle);
}

function v2AssertBranchExists(branch) {
    const normalized = v2NormalizeBranch(branch);
    if (!normalized) v2Fail('Branch is required for prepare step');
    try {
        execSync(`git show-ref --verify --quiet "refs/heads/${normalized}"`);
    } catch {
        v2Fail('Branch does not exist locally; create it first.', [
            `branch=${normalized}`,
            `Suggested: just worktree-add ${wpId} main ${normalized}`,
        ]);
    }
}

if (action === 'refine') {
    v2AssertWpId(wpId);

    const refinementPath = (data && fs.existsSync(data)) ? data : defaultRefinementPath(wpId);
    const validation = validateRefinementFile(refinementPath, { expectedWpId: wpId, requireSignature: false });
    if (!validation.ok) {
        v2Fail(`Refinement is not ready for review: ${refinementPath}`, validation.errors);
    }

    let resolved = null;
    try {
        resolved = resolveSpecCurrent();
    } catch {
        // validateRefinementFile already reports this.
    }

    state.gate_logs.push({
        wpId,
        type: 'REFINEMENT',
        refinement_path: refinementPath.replace(/\\/g, '/'),
        spec_target_resolved: resolved ? `.GOV/roles_shared/SPEC_CURRENT.md -> ${resolved.specFileName}` : '.GOV/roles_shared/SPEC_CURRENT.md -> <unresolved>',
        spec_target_sha1: resolved ? resolved.sha1 : '<unresolved>',
        timestamp: new Date().toISOString(),
    });
    saveState(state);

    v2PrintGateBlocks({
        wpId,
        stage: 'REFINEMENT',
        next: 'SIGNATURE',
        operatorAction: `Collect explicit approval + one-time signature for ${wpId}`,
        gateRan: `just record-refinement ${wpId}`,
        result: 'PASS',
        why: 'Technical refinement recorded; request explicit user approval + one-time signature before proceeding.',
        gateOutputLines: [
            `[ORCHESTRATOR GATE] Technical Refinement recorded for ${wpId}.`,
        ],
        nextCommands: [
            `# Present the Technical Refinement Block in chat.`,
            `# When approved, set USER_APPROVAL_EVIDENCE in the refinement file to: APPROVE REFINEMENT ${wpId}`,
            `just record-signature ${wpId} {usernameDDMMYYYYHHMM}`,
        ],
    });
    process.exit(0);
}

if (action === 'sign') {
    v2AssertWpId(wpId);
    const signature = data;
    if (!signature || !/^[a-z]+[0-9]{12}$/.test(signature)) {
        v2Fail('Invalid signature format. Expected {username}{DDMMYYYYHHMM}');
    }

    const lastRefinement = v2ResolveLastRefinement();
    if (!lastRefinement) {
        v2Fail(`No technical refinement recorded for ${wpId}. Run: just record-refinement ${wpId}`);
    }

    const lastSignature = v2ResolveLastSignature();
    if (lastSignature) {
        v2Fail(`A signature is already recorded for ${wpId} (${lastSignature.signature}). Create a new WP variant instead of re-signing.`);
    }

    const now = new Date();

    const refinementPath = lastRefinement.refinement_path || defaultRefinementPath(wpId);
    const refinementValidation = validateRefinementFile(refinementPath, { expectedWpId: wpId, requireSignature: false });
    if (!refinementValidation.ok) {
        v2Fail(`Refinement is not complete; cannot sign: ${refinementPath}`, refinementValidation.errors);
    }

    // HARD GATE: Do not consume a one-time signature for WP packet approval if refinement requires enrichment.
    try {
        const refinementContent = fs.readFileSync(refinementPath, 'utf8');
        const m = refinementContent.match(/^\s*-\s*ENRICHMENT_NEEDED\s*:\s*(YES|NO)\s*$/mi);
        const enrichmentNeeded = (m?.[1] || '').toUpperCase();
        if (enrichmentNeeded === 'YES') {
            v2Fail('Refinement declares ENRICHMENT_NEEDED=YES; packet signature is forbidden.', [
                'Run the spec enrichment workflow first (new spec version + update .GOV/roles_shared/SPEC_CURRENT.md).',
                'Then create a NEW WP variant anchored to the updated spec (new WP_ID; new one-time signature).',
            ]);
        }
    } catch (e) {
        v2Fail(`Failed to read refinement file: ${refinementPath}`, [String(e?.message || e)]);
    }

    // HARD GATE: signature requires explicit user approval evidence in the refinement file.
    // This is intentionally deterministic (not time-based) to prevent "sleep" bypass.
    try {
        const refinementContent = fs.readFileSync(refinementPath, "utf8");
        const approvalEvidence = v2GetSingleField(refinementContent, "USER_APPROVAL_EVIDENCE");
        const expected = `APPROVE REFINEMENT ${wpId}`;
        if (!approvalEvidence || approvalEvidence === "<pending>") {
            v2Fail("Missing USER_APPROVAL_EVIDENCE in refinement; cannot consume one-time signature.", [
                `Add a line to ${refinementPath.replace(/\\/g, "/")} under METADATA:`,
                `- USER_APPROVAL_EVIDENCE: ${expected}`,
            ]);
        }
        if (approvalEvidence !== expected) {
            v2Fail("Invalid USER_APPROVAL_EVIDENCE in refinement; cannot consume one-time signature.", [
                `Expected: ${expected}`,
                `Got: ${approvalEvidence}`,
            ]);
        }
    } catch (e) {
        v2Fail(`Failed to verify USER_APPROVAL_EVIDENCE in refinement: ${refinementPath}`, [String(e?.message || e)]);
    }

    // Refinement must not already be signed.
    try {
        const existing = fs.readFileSync(refinementPath, 'utf8');
        const existingSig = v2GetSingleField(existing, 'USER_SIGNATURE');
        if (existingSig && existingSig !== '<pending>') {
            v2Fail(`Refinement already has a USER_SIGNATURE (${existingSig}); signatures are one-time use.`);
        }
    } catch (e) {
        v2Fail(`Failed to read refinement file: ${refinementPath}`, [String(e?.message || e)]);
    }

    // One-time signature guard: refuse if it appears anywhere in tracked repo files.
    const grepHit = v2GitGrepOrEmpty(signature);
    if (grepHit) {
        v2Fail('Signature already appears in repo (one-time use). Provide a NEW signature.', [grepHit]);
    }

    // Update refinement file to reflect approval.
    try {
        const refinementContent = fs.readFileSync(refinementPath, 'utf8');
        const updated = refinementContent
            .replace(/^\s*-\s*USER_REVIEW_STATUS\s*:\s*.*$/mi, '- USER_REVIEW_STATUS: APPROVED')
            .replace(/^\s*-\s*USER_SIGNATURE\s*:\s*.*$/mi, `- USER_SIGNATURE: ${signature}`);
        fs.writeFileSync(refinementPath, updated, 'utf8');
    } catch (e) {
        v2Fail(`Failed to update refinement file with signature: ${refinementPath}`, [String(e?.message || e)]);
    }

    // Append to SIGNATURE_AUDIT (protocol requirement).
    if (!fs.existsSync(SIGNATURE_AUDIT_PATH)) {
        v2Fail(`Missing signature audit file: ${SIGNATURE_AUDIT_PATH}`);
    }

    try {
        const resolved = resolveSpecCurrent();
        const audit = fs.readFileSync(SIGNATURE_AUDIT_PATH, 'utf8');
        if (audit.includes(`| ${signature} |`)) {
            v2Fail('Signature already present in SIGNATURE_AUDIT (one-time use). Provide a NEW signature.');
        }

        const ts = signature.slice(-12);
        const dd = ts.slice(0, 2);
        const mm = ts.slice(2, 4);
        const yyyy = ts.slice(4, 8);
        const hh = ts.slice(8, 10);
        const min = ts.slice(10, 12);
        const dateTime = `${yyyy}-${mm}-${dd} ${hh}:${min}`;
        const verMatch = resolved.specFileName.match(/v([0-9.]+)\.md/);
        const specVer = verMatch ? `v${verMatch[1]}` : resolved.specFileName;

        const row = `| ${signature} | Orchestrator | ${dateTime} | Task packet creation: ${wpId} | ${specVer} | Approved after Technical Refinement (see ${refinementPath.replace(/\\\\/g, '/')} ). |`;
        const updatedAudit = v2InsertSignatureAuditRow(audit, row);
        if (!updatedAudit) {
            v2Fail('SIGNATURE_AUDIT format changed; cannot append deterministically.');
        }
        fs.writeFileSync(SIGNATURE_AUDIT_PATH, updatedAudit, 'utf8');
    } catch (e) {
        v2Fail('Failed to append to .GOV/roles_shared/SIGNATURE_AUDIT.md', [String(e?.message || e)]);
    }

    state.gate_logs.push({
        wpId,
        type: 'SIGNATURE',
        signature,
        timestamp: now.toISOString(),
        refinement_path: refinementPath.replace(/\\/g, '/'),
    });
    saveState(state);

    v2PrintGateBlocks({
        wpId,
        stage: 'SIGNATURE',
        next: 'PACKET_CREATE',
        operatorAction: 'NONE',
        gateRan: `just record-signature ${wpId} ${signature}`,
        result: 'PASS',
        why: 'One-time signature recorded + audited; proceed to worktree + packet creation. Record PREPARE before delegation/pre-work.',
        gateOutputLines: [
            `[ORCHESTRATOR GATE] Signature recorded for ${wpId}.`,
        ],
        nextCommands: [
            `just orchestrator-worktree-and-packet ${wpId}`,
            `# Before delegation/pre-work:`,
            `just record-prepare ${wpId} {Coder-A|Coder-B}`,
            `just pre-work ${wpId}`,
            `# Alternative (all-in-one, includes PREPARE): just orchestrator-prepare-and-packet ${wpId} {Coder-A|Coder-B}`,
        ],
    });
    process.exit(0);
}

if (action === 'prepare') {
    v2AssertWpId(wpId);

    const coderId = (argvData[0] || '').trim();
    const branch = v2NormalizeBranch((argvData[1] || `feat/${wpId}`).trim());
    const worktreeDir = (argvData[2] || `../wt-${wpId}`).trim();

    const isAbsoluteWorktreeDir =
        /^[A-Za-z]:[\\/]/.test(worktreeDir) || worktreeDir.startsWith('\\\\') || worktreeDir.startsWith('//');
    if (isAbsoluteWorktreeDir) {
        v2Fail('worktree_dir must be repo-relative (drive-agnostic).', [
            `got=${worktreeDir}`,
            'Recommended: omit worktree_dir to use default ../wt-<WP_ID>, or pass a relative path like ../wt-WP-...',
        ]);
    }

    if (!coderId) {
        v2Fail('Missing coder assignment. Usage: just record-prepare WP-... Coder-A [branch] [worktree_dir]');
    }

    const lastSignature = v2ResolveLastSignature();
    if (!lastSignature) {
        v2Fail(`No signature recorded for ${wpId}. Run: just record-signature ${wpId} {usernameDDMMYYYYHHMM}`);
    }

    const lastPrepare = v2ResolveLastPrepare();
    if (lastPrepare) {
        console.warn(`[GATE WARNING] A prepare record already exists for ${wpId}; appending a new prepare entry.`);
    }

    v2AssertBranchExists(branch);
    if (!v2WorktreeHasBranch(branch)) {
        v2Fail('WP worktree not found for branch (required before task packet creation).', [
            `branch=${branch}`,
            'Create it first with: just worktree-add ' + wpId,
        ]);
    }

    state.gate_logs.push({
        wpId,
        type: 'PREPARE',
        coder_id: coderId,
        branch,
        worktree_dir: worktreeDir.replace(/\\/g, '/'),
        timestamp: new Date().toISOString(),
    });
    saveState(state);

    v2PrintGateBlocks({
        wpId,
        stage: 'PREPARE',
        next: 'PACKET_CREATE',
        operatorAction: 'NONE',
        gateRan: `just record-prepare ${wpId} ${coderId} ${branch} ${worktreeDir}`,
        result: 'PASS',
        why: 'WP worktree/branch + coder assignment recorded; task packet creation is now unblocked.',
        gateOutputLines: [
            `[ORCHESTRATOR GATE] Prepared ${wpId} for development.`,
            `- coder_id: ${coderId}`,
            `- branch: ${branch}`,
            `- worktree_dir: ${worktreeDir}`,
        ],
        nextCommands: [
            `just create-task-packet ${wpId}`,
        ],
    });
    process.exit(0);
}

v2Fail('Unknown action. Expected: refine|sign|prepare');
