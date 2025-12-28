import fs from 'node:fs';
import path from 'node:path';

/**
 * [CX-GATE-001] Binary Phase Gate Validator
 * Enforces ordered phases and prevents merged turns.
 */

const wpId = process.argv[2];
if (!wpId) {
    console.error("Usage: node gate-check.mjs <WP_ID>");
    process.exit(1);
}

const wpPath = path.join(process.cwd(), 'docs', 'task_packets', `${wpId}.md`);
if (!fs.existsSync(wpPath)) {
    console.error(`? GATE FAIL: Task Packet ${wpId}.md not found.`);
    process.exit(1);
}

const content = fs.readFileSync(wpPath, 'utf8');

const markers = [
    { id: "BOOTSTRAP", pattern: /#+ BOOTSTRAP/i },
    { id: "SKELETON", pattern: /#+ SKELETON/i },
    { id: "APPROVAL", pattern: /SKELETON APPROVED/i }
];

const missing = markers.filter(m => !m.pattern.test(content));

console.log(`Checking Phase Gate for ${wpId}...`);

const indexOf = (pattern) => {
    const match = content.match(pattern);
    return match && typeof match.index === "number" ? match.index : -1;
};

const order = {
    BOOTSTRAP: indexOf(/#+ BOOTSTRAP/i),
    SKELETON: indexOf(/#+ SKELETON/i),
    APPROVAL: indexOf(/SKELETON APPROVED/i),
};

// Validation 1: Mandatory checkpoints for "In Progress"
if (content.includes("Status: In-Progress") || content.includes("Status: In Progress")) {
    if (!content.match(/#+ BOOTSTRAP/i)) {
        console.error("? GATE FAIL: 'In Progress' status requires a BOOTSTRAP block.");
        process.exit(1);
    }
}

// Validation 2: Interface-First Invariant [CX-625]
const implementationStarted = content.includes("## VALIDATION (Coder)") || content.includes("## VALIDATION REPORT");
if (implementationStarted && !content.match(/SKELETON APPROVED/i)) {
    console.error("? GATE FAIL: Implementation detected without SKELETON APPROVED marker.");
    process.exit(1);
}

// Validation 3: Anti-Turn-Merging (Heuristic)
if (missing.length > 0 && implementationStarted) {
    console.error(`? GATE FAIL: Missing mandatory phases: ${missing.map(m => m.id).join(', ')}`);
    process.exit(1);
}

// Validation 4: Enforce sequence order (BOOTSTRAP -> SKELETON -> APPROVAL)
if (order.BOOTSTRAP === -1 || order.SKELETON === -1) {
    console.error("? GATE FAIL: Missing BOOTSTRAP or SKELETON markers.");
    process.exit(1);
}
if (order.BOOTSTRAP > order.SKELETON) {
    console.error("? GATE FAIL: SKELETON appears before BOOTSTRAP.");
    process.exit(1);
}
if (order.APPROVAL !== -1 && order.SKELETON > order.APPROVAL) {
    console.error("? GATE FAIL: SKELETON APPROVED marker must follow SKELETON.");
    process.exit(1);
}

console.log("? GATE PASS: Workflow sequence verified.");
