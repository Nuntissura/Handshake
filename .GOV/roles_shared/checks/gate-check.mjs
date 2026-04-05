import fs from 'node:fs';
import { workPacketAbsPath } from '../scripts/lib/runtime-paths.mjs';

/**
 * [CX-GATE-001] Binary Phase Gate Validator
 * Enforces ordered phases and prevents merged turns.
 *
 * Hardened per WP-1-Gate-Check-Tool-v2:
 * - Line-based parsing with fenced code block tracking
 * - Detects phases via heading lines only (outside fences)
 */

const wpId = process.argv[2];
if (!wpId) {
    console.error("Usage: node gate-check.mjs <WP_ID>");
    process.exit(1);
}

const wpPath = workPacketAbsPath(wpId);
if (!fs.existsSync(wpPath)) {
  console.error(`? GATE FAIL: Work Packet ${wpId} not found at the resolved packet path.`);
  process.exit(1);
}

const content = fs.readFileSync(wpPath, 'utf8');

/**
 * Parse content line-by-line, tracking fenced code block state.
 * Returns positions of valid markers found OUTSIDE code fences only.
 *
 * @param {string} content - The markdown content to parse
 * @returns {Object} ParseResult with marker positions and flags
 */
function parseMarkersFromContent(content) {
    const lines = content.split('\n');
    let inCodeFence = false;

    const result = {
        bootstrapHeadingLine: -1,
        skeletonHeadingLine: -1,
        implementationDetected: false,
        statusInProgress: false
    };

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const trimmed = line.trim();

        // Toggle fence state on ``` lines (trimmed line starts with ```)
        if (trimmed.startsWith('```')) {
            inCodeFence = !inCodeFence;
            continue;
        }

        // Skip all marker detection inside fenced code blocks
        if (inCodeFence) continue;

        // Detect BOOTSTRAP heading (heading syntax only, outside fence)
        if (/^#{1,6}\s+BOOTSTRAP\b/i.test(line)) {
            if (result.bootstrapHeadingLine === -1) {
                result.bootstrapHeadingLine = i;
            }
        }

        // Detect SKELETON heading (heading syntax only, outside fence)
        if (/^#{1,6}\s+SKELETON\b/i.test(line)) {
            if (result.skeletonHeadingLine === -1) {
                result.skeletonHeadingLine = i;
            }
        }

        // Detect implementation evidence (heading syntax only, outside fence)
        if (/^#{1,6}\s+VALIDATION\s*\(Coder\)/i.test(line) ||
            /^#{1,6}\s+VALIDATION REPORT\b/i.test(line)) {
            result.implementationDetected = true;
        }

        // Detect status (outside fence)
        if (/Status:\s*In[- ]?Progress/i.test(line)) {
            result.statusInProgress = true;
        }
    }

    return result;
}

// Parse the content
const parsed = parseMarkersFromContent(content);

console.log(`Checking Phase Gate for ${wpId}...`);

// Validation 1: Mandatory checkpoints for "In Progress"
if (parsed.statusInProgress && parsed.bootstrapHeadingLine === -1) {
    console.error("? GATE FAIL: 'In Progress' status requires a BOOTSTRAP block.");
    process.exit(1);
}

// Validation 2: Anti-Turn-Merging (Heuristic)
const missingPhases = [];
if (parsed.bootstrapHeadingLine === -1) missingPhases.push('BOOTSTRAP');
if (parsed.skeletonHeadingLine === -1) missingPhases.push('SKELETON');

if (missingPhases.length > 0 && parsed.implementationDetected) {
    console.error(`? GATE FAIL: Missing mandatory phases: ${missingPhases.join(', ')}`);
    process.exit(1);
}

// Validation 3: Enforce sequence order (BOOTSTRAP -> SKELETON)
if (parsed.bootstrapHeadingLine === -1 || parsed.skeletonHeadingLine === -1) {
    console.error("? GATE FAIL: Missing BOOTSTRAP or SKELETON markers.");
    process.exit(1);
}
if (parsed.bootstrapHeadingLine > parsed.skeletonHeadingLine) {
    console.error("? GATE FAIL: SKELETON appears before BOOTSTRAP.");
    process.exit(1);
}

console.log("? GATE PASS: Workflow sequence verified.");
