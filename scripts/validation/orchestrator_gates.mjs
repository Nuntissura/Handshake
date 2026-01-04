import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import {
    defaultRefinementPath,
    resolveSpecCurrent,
    validateRefinementFile,
} from './refinement-check.mjs';

const STATE_FILE = 'docs/ORCHESTRATOR_GATES.json';

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

const SIGNATURE_AUDIT_PATH = path.join('docs', 'SIGNATURE_AUDIT.md');

function v2Fail(msg, details = []) {
    console.error(`[GATE ERROR] ${msg}`);
    details.forEach((d) => console.error(`- ${d}`));
    process.exit(1);
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
        spec_target_resolved: resolved ? `docs/SPEC_CURRENT.md -> ${resolved.specFileName}` : 'docs/SPEC_CURRENT.md -> <unresolved>',
        spec_target_sha1: resolved ? resolved.sha1 : '<unresolved>',
        timestamp: new Date().toISOString(),
        turn_token: String(Date.now()),
    });
    saveState(state);

    console.log(`[ORCHESTRATOR GATE] Technical Refinement recorded for ${wpId}.`);
    console.log('[GATE LOCKED] This is the refinement phase; do not request/record USER_SIGNATURE in this turn.');
    console.log('[NEXT] Wait for explicit user review, then run: just record-signature ' + wpId + ' {usernameDDMMYYYYHHMM}');
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

    const refDate = new Date(lastRefinement.timestamp);
    const now = new Date();
    const diffSeconds = (now.getTime() - refDate.getTime()) / 1000;
    if (diffSeconds < 10) {
        v2Fail('Automation momentum detected: refinement and signature recorded too close together.', [
            `diff_seconds=${diffSeconds}`,
            'Protocol requires a standalone user review turn between refinement and signature.',
        ]);
    }

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
                'Run the spec enrichment workflow first (new spec version + update docs/SPEC_CURRENT.md).',
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
        v2Fail('Failed to append to docs/SIGNATURE_AUDIT.md', [String(e?.message || e)]);
    }

    state.gate_logs.push({
        wpId,
        type: 'SIGNATURE',
        signature,
        timestamp: now.toISOString(),
        refinement_path: refinementPath.replace(/\\/g, '/'),
    });
    saveState(state);

    console.log(`[ORCHESTRATOR GATE] Signature recorded for ${wpId}.`);
    console.log('[GATE PARTIAL] Signature recorded. Next, you MUST create a WP branch/worktree and record assignment before creating the Task Packet.');
    console.log(`[NEXT] 1) Create WP worktree: just worktree-add ${wpId}`);
    console.log(`[NEXT] 2) Record assignment: just record-prepare ${wpId} {Coder-A|Coder-B} (optional: {branch} {worktree_dir})`);
    console.log(`[NEXT] 3) Then create packet: just create-task-packet ${wpId}`);
    process.exit(0);
}

if (action === 'prepare') {
    v2AssertWpId(wpId);

    const coderId = (argvData[0] || '').trim();
    const branch = v2NormalizeBranch((argvData[1] || `feat/${wpId}`).trim());
    const worktreeDir = (argvData[2] || `../wt-${wpId}`).trim();

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

    console.log(`[ORCHESTRATOR GATE] Prepared ${wpId} for development.`);
    console.log(`- coder_id: ${coderId}`);
    console.log(`- branch: ${branch}`);
    console.log(`- worktree_dir: ${worktreeDir}`);
    console.log('[NEXT] Create packet: just create-task-packet ' + wpId);
    process.exit(0);
}

if (action !== 'refine' && action !== 'sign') {
    v2Fail('Unknown action. Expected: refine|sign|prepare');
}

if (action === 'refine') {
    // data is an optional hash or description of the refinement
    const refinementEntry = {
        wpId,
        type: 'REFINEMENT',
        data: data || 'No detail provided',
        timestamp: new Date().toISOString(),
        // We use a simple counter to track "Protocol Turns"
        turn_token: Math.random().toString(36).substring(7)
    };
    
    state.gate_logs.push(refinementEntry);
    saveState(state);
    console.log(`
✅ [ORCHESTRATOR GATE] Technical Refinement recorded for ${wpId}.`);
    console.log(`🔒 [GATE LOCKED] You must wait for a new turn to provide a signature.
`);
}

if (action === 'sign') {
    // data is the signature: usernameDDMMYYYYHHMM
    if (!data || !/^[a-z]+[0-9]{12}$/.test(data)) {
        console.error(`
❌ [GATE ERROR] Invalid signature format. Expected {username}{DDMMYYYYHHMM}
`);
        process.exit(1);
    }

    const logs = state.gate_logs.filter(l => l.wpId === wpId);
    const lastRefinement = [...logs].reverse().find(l => l.type === 'REFINEMENT');
    
    if (!lastRefinement) {
        console.error(`
❌ [GATE ERROR] No technical refinement found for ${wpId}. You cannot sign what hasn't been refined.
`);
        process.exit(1);
    }

    // BLOCK: Automation Momentum Detection
    // If the signature's HHMM matches the refinement's HHMM, it's likely a merged turn.
    const refDate = new Date(lastRefinement.timestamp);
    const refHHMM = `${String(refDate.getDate()).padStart(2, '0')}${String(refDate.getMonth() + 1).padStart(2, '0')}${refDate.getFullYear()}${String(refDate.getHours()).padStart(2, '0')}${String(refDate.getMinutes()).padStart(2, '0')}`;
    const sigHHMM = data.slice(-12);

    // If the refinement was recorded less than 10 seconds ago, it's definitely the same turn.
    const now = new Date();
    const diffSeconds = (now.getTime() - refDate.getTime()) / 1000;

    if (diffSeconds < 10) {
        console.error(`
🚨 [GATE ERROR: AUTOMATION MOMENTUM]`);
        console.error(`Refinement and Signature detected in the same turn (diff: ${diffSeconds}s).`);
        console.error(`The protocol mandates a standalone turn for refinement inspection.`);
        console.error(`STOP and wait for the user to review the refinement in a NEW turn.
`);
        process.exit(1);
    }

    state.gate_logs.push({
        wpId,
        type: 'SIGNATURE',
        signature: data,
        timestamp: now.toISOString()
    });
    
    saveState(state);
    console.log(`
✅ [ORCHESTRATOR GATE] Signature validated for ${wpId}.`);
    console.log(`🔓 [GATE UNLOCKED] You may now create the Task Packet.
`);
}