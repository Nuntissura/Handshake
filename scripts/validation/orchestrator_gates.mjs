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
const data = process.argv[4];

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
    console.log('[GATE UNLOCKED] You may now create the Task Packet.');
    process.exit(0);
}

if (action !== 'refine' && action !== 'sign') {
    v2Fail('Unknown action. Expected: refine|sign');
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
