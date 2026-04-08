import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { workPacketAbsPath } from '../scripts/lib/runtime-paths.mjs';

/**
 * [CX-GATE-001] Binary Phase Gate Validator
 * Enforces ordered phases and prevents merged turns.
 *
 * Hardened per WP-1-Gate-Check-Tool-v2:
 * - Line-based parsing with fenced code block tracking
 * - Detects phases via heading lines only (outside fences)
 */

/**
 * Parse content line-by-line, tracking fenced code block state.
 * Returns positions of valid markers found OUTSIDE code fences only.
 *
 * @param {string} content - The markdown content to parse
 * @returns {Object} ParseResult with marker positions and flags
 */
export function parseMarkersFromContent(content) {
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

export function runGateCheck(wpId) {
    const normalizedWpId = String(wpId || '').trim();
    if (!normalizedWpId) {
        return {
            ok: false,
            output: "Usage: node gate-check.mjs <WP_ID>\n",
        };
    }

    const wpPath = workPacketAbsPath(normalizedWpId);
    if (!fs.existsSync(wpPath)) {
        return {
            ok: false,
            output: `? GATE FAIL: Work Packet ${normalizedWpId} not found at the resolved packet path.\n`,
        };
    }

    const content = fs.readFileSync(wpPath, 'utf8');
    const parsed = parseMarkersFromContent(content);
    const lines = [`Checking Phase Gate for ${normalizedWpId}...`];

    if (parsed.statusInProgress && parsed.bootstrapHeadingLine === -1) {
        lines.push("? GATE FAIL: 'In Progress' status requires a BOOTSTRAP block.");
        return { ok: false, output: `${lines.join('\n')}\n` };
    }

    const missingPhases = [];
    if (parsed.bootstrapHeadingLine === -1) missingPhases.push('BOOTSTRAP');
    if (parsed.skeletonHeadingLine === -1) missingPhases.push('SKELETON');

    if (missingPhases.length > 0 && parsed.implementationDetected) {
        lines.push(`? GATE FAIL: Missing mandatory phases: ${missingPhases.join(', ')}`);
        return { ok: false, output: `${lines.join('\n')}\n` };
    }

    if (parsed.bootstrapHeadingLine === -1 || parsed.skeletonHeadingLine === -1) {
        lines.push("? GATE FAIL: Missing BOOTSTRAP or SKELETON markers.");
        return { ok: false, output: `${lines.join('\n')}\n` };
    }
    if (parsed.bootstrapHeadingLine > parsed.skeletonHeadingLine) {
        lines.push("? GATE FAIL: SKELETON appears before BOOTSTRAP.");
        return { ok: false, output: `${lines.join('\n')}\n` };
    }

    lines.push("? GATE PASS: Workflow sequence verified.");
    return { ok: true, output: `${lines.join('\n')}\n` };
}

function runCli(argv = process.argv.slice(2)) {
    const wpId = argv[0];
    if (!wpId) {
        console.error("Usage: node gate-check.mjs <WP_ID>");
        process.exit(1);
    }

    const result = runGateCheck(wpId);
    process.stdout.write(result.output);
    process.exit(result.ok ? 0 : 1);
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) runCli();
