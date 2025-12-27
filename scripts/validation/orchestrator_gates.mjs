import fs from 'fs';
import path from 'path';

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
