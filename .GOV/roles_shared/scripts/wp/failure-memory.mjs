#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  ensureGovernanceRuntimeDir,
} from "../lib/runtime-paths.mjs";

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

const FAILURE_MEMORY_DIR = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared");
const FAILURE_MEMORY_FILE = path.join(FAILURE_MEMORY_DIR, "FAILURE_MEMORY.json");

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function loadEntries() {
  if (!fs.existsSync(FAILURE_MEMORY_FILE)) return [];
  try {
    const raw = fs.readFileSync(FAILURE_MEMORY_FILE, "utf8");
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveEntries(entries) {
  ensureGovernanceRuntimeDir("roles_shared");
  fs.writeFileSync(FAILURE_MEMORY_FILE, JSON.stringify(entries, null, 2) + "\n", "utf8");
}

// ---------------------------------------------------------------------------
// record mode
// ---------------------------------------------------------------------------

function doRecord(errorCategory, fileSurface, errorPattern, fixPattern, wpId) {
  const entries = loadEntries();

  const existing = entries.find(
    (e) => e.file_surface === fileSurface && e.error_pattern === errorPattern,
  );

  if (existing) {
    existing.occurrences = (existing.occurrences || 1) + 1;
    existing.recorded_at = new Date().toISOString();
    if (wpId) existing.wp_id = wpId;
    saveEntries(entries);
    console.log(`[failure-memory] Incremented occurrences (now ${existing.occurrences}) for existing pattern on ${fileSurface}`);
    return;
  }

  entries.push({
    error_category: errorCategory,
    file_surface: fileSurface,
    error_pattern: errorPattern,
    fix_pattern: fixPattern,
    wp_id: wpId || "",
    recorded_at: new Date().toISOString(),
    occurrences: 1,
  });

  saveEntries(entries);
  console.log(`[failure-memory] Recorded new failure pattern for ${fileSurface}`);
}

// ---------------------------------------------------------------------------
// query mode
// ---------------------------------------------------------------------------

function doQuery(queryText) {
  const entries = loadEntries();
  const lowerQuery = queryText.toLowerCase();

  const matches = entries.filter(
    (e) =>
      (e.file_surface || "").toLowerCase().includes(lowerQuery) ||
      (e.error_pattern || "").toLowerCase().includes(lowerQuery),
  );

  if (matches.length === 0) {
    console.log(`[failure-memory] No matches for "${queryText}"`);
    return;
  }

  console.log(`[failure-memory] ${matches.length} match(es) for "${queryText}":\n`);
  for (const m of matches) {
    console.log(`  Category:    ${m.error_category}`);
    console.log(`  File:        ${m.file_surface}`);
    console.log(`  Error:       ${m.error_pattern}`);
    console.log(`  Fix:         ${m.fix_pattern}`);
    console.log(`  WP:          ${m.wp_id || "(none)"}`);
    console.log(`  Occurrences: ${m.occurrences}`);
    console.log(`  Recorded:    ${m.recorded_at}`);
    console.log("");
  }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const [mode, ...rest] = process.argv.slice(2);

if (mode === "record") {
  const [errorCategory, fileSurface, errorPattern, fixPattern, wpId] = rest;
  if (!errorCategory || !fileSurface || !errorPattern || !fixPattern) {
    console.error("Usage: failure-memory.mjs record <error_category> <file_surface> <error_pattern> <fix_pattern> [wp_id]");
    process.exit(1);
  }
  doRecord(errorCategory, fileSurface, errorPattern, fixPattern, wpId || "");
} else if (mode === "query") {
  const [queryText] = rest;
  if (!queryText) {
    console.error("Usage: failure-memory.mjs query <file_surface_or_error_text>");
    process.exit(1);
  }
  doQuery(queryText);
} else {
  console.error("Usage: failure-memory.mjs <record|query> ...");
  process.exit(1);
}
