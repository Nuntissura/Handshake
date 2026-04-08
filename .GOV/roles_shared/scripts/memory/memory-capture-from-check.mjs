/**
 * memory-capture-from-check.mjs
 *
 * Auto-captures check findings (failures, warnings) into governance memory
 * so future sessions can learn from past check outcomes.
 *
 * This is a best-effort service — if memory capture fails, checks still proceed.
 */

import { openGovernanceMemoryDb, addMemory, closeDb } from './governance-memory-lib.mjs';

/**
 * @param {{ check: string, findings: string[], wpId?: string }} opts
 */
export function captureCheckFindings({ check, findings, wpId }) {
  if (!findings || findings.length === 0) return;
  let db;
  try {
    db = openGovernanceMemoryDb();
    const content = findings.join('; ').slice(0, 500);
    addMemory(db, {
      memoryType: 'procedural',
      content: `[${check}] ${wpId ? `WP ${wpId}: ` : ''}${content}`,
      importance: 0.6,
      wpId: wpId || '',
      tags: ['check-finding', check, wpId].filter(Boolean).join(','),
    });
  } catch {
    // best-effort — do not block checks on memory failures
  } finally {
    if (db) closeDb(db);
  }
}

/**
 * Compatibility shim for older checks that emit one finding at a time.
 *
 * @param {{ check: string, finding: string, wpId?: string }} opts
 */
export function captureCheckFinding({ check, finding, wpId }) {
  if (!finding) return;
  captureCheckFindings({ check, findings: [finding], wpId });
}
