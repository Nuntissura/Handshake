/**
 * RGF-132/G5: Best-effort memory capture from governance check failures.
 *
 * Called by validator checks and coder post-work when findings are detected.
 * Persists the finding as a procedural or semantic memory so future sessions
 * benefit from the pattern.
 *
 * Usage:
 *   import { captureCheckFinding } from "../memory/memory-capture-from-check.mjs";
 *   captureCheckFinding({ check, finding, wpId, fileScope, memoryType });
 *
 * Fails silently — check failures must not be blocked by memory write errors.
 */

import {
  openGovernanceMemoryDb,
  closeDb,
  addMemory,
} from "./governance-memory-lib.mjs";

export function captureCheckFinding({
  check = "",
  finding = "",
  wpId = "",
  fileScope = "",
  memoryType = "procedural",
  importance = 0.6,
} = {}) {
  try {
    const { db } = openGovernanceMemoryDb();
    try {
      const topic = `${check}: ${finding.slice(0, 60)}`;
      // Dedup: skip if this exact topic+wp+type already exists
      const existing = db.prepare(
        "SELECT id FROM memory_index WHERE topic = ? AND wp_id = ? AND memory_type = ?"
      ).get(topic, wpId, memoryType);
      if (existing) return;

      addMemory(db, {
        memoryType,
        topic,
        summary: finding.slice(0, 200),
        wpId,
        fileScope,
        importance,
        content: finding,
        sourceArtifact: check,
        sourceRole: "VALIDATOR",
        metadata: { captured_from_check: true, check_name: check },
      });
    } finally { closeDb(db); }
  } catch {
    // Best-effort: memory capture failure must not block the check
  }
}

export function captureCheckFindings({
  check = "",
  findings = [],
  wpId = "",
  memoryType = "procedural",
  importance = 0.6,
} = {}) {
  for (const finding of findings.slice(0, 5)) {
    captureCheckFinding({ check, finding: String(finding), wpId, memoryType, importance });
  }
}
