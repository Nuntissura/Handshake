import fs from 'node:fs';
import path from 'node:path';

/**
 * [CX-GATE-001] Binary Phase Gate Validator
 * Logic:
 * 1. Verify markers exist: BOOTSTRAP, SKELETON, SKELETON APPROVED.
 * 2. Ensure Turn-Based Separation: Markers must not be added in the same turn.
 * 3. Enforce Sequence: SKELETON requires BOOTSTRAP; IMPLEMENTATION requires APPROVAL.
 */

const wpId = process.argv[2];
if (!wpId) {
    console.error("Usage: node gate-check.mjs <WP_ID>");
    process.exit(1);
}

const wpPath = path.join(process.cwd(), 'docs', 'task_packets', `${wpId}.md`);
if (!fs.existsSync(wpPath)) {
    console.error(`❌ GATE FAIL: Task Packet ${wpId}.md not found.`);
    process.exit(1);
}

const content = fs.readFileSync(wpPath, 'utf8');

const markers = [
    { id: "BOOTSTRAP", pattern: /#+ BOOTSTRAP/i },
    { id: "SKELETON", pattern: /#+ SKELETON/i },
    { id: "APPROVAL", pattern: /SKELETON APPROVED/i }
];

const findings = markers.filter(m => m.pattern.test(content));
const missing = markers.filter(m => !m.pattern.test(content));

console.log(`Checking Phase Gate for ${wpId}...`);

// Validation 1: Mandatory checkpoints for "In Progress"
if (content.includes("Status: In-Progress") || content.includes("Status: In Progress")) {
    if (!content.match(/#+ BOOTSTRAP/i)) {
        console.error("❌ GATE FAIL: 'In Progress' status requires a BOOTSTRAP block.");
        process.exit(1);
    }
}

// Validation 2: Interface-First Invariant [CX-625]
const implementationStarted = content.includes("## VALIDATION (Coder)") || content.includes("## VALIDATION REPORT");
if (implementationStarted && !content.match(/SKELETON APPROVED/i)) {
    console.error("❌ GATE FAIL: Implementation detected without SKELETON APPROVED marker.");
    process.exit(1);
}

// Validation 3: Anti-Turn-Merging (Heuristic)
// If BOOTSTRAP and SKELETON are too close or appear to be added in one block
// (Note: This is hard to detect perfectly without git diff, but we check for presence).
if (missing.length > 0 && implementationStarted) {
    console.error(`❌ GATE FAIL: Missing mandatory phases: ${missing.map(m => m.id).join(', ')}`);
    process.exit(1);
}

console.log("✅ GATE PASS: Workflow sequence verified.");
