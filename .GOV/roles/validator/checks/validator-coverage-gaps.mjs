#!/usr/bin/env node
/**
 * Coverage sanity helper:
 * - Ensures there is at least some test coverage in target paths.
 * - Intended as a quick guard that changed areas are protected; not a full coverage tool.
 *
 * Exits non-zero if no tests are detected in the given targets.
 */
import { execSync } from "node:child_process";

const targets = process.argv.slice(2);
const defaultTargets = [
  "src/backend/handshake_core/src",
  "src/backend/handshake_core/tests",
  "tests",
  "app/src",
];

const scopes = targets.length > 0 ? targets : defaultTargets;

const patterns = [
  { label: "rust_tests", pattern: "#\\[test\\]" },
  { label: "ts_tests", pattern: "describe\\(" },
  { label: "ts_it", pattern: "\\bit\\(" },
];

function runRg(pattern) {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${scopes.join(
    " "
  )}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    console.error(`validator-coverage-gaps: scan failed: ${err.message}`);
    process.exit(1);
  }
}

const hits = [];
for (const pat of patterns) {
  const out = runRg(pat.pattern);
  if (out) {
    hits.push({ label: pat.label, lines: out.split("\n").length });
  }
}

if (hits.length === 0) {
  console.error(
    `validator-coverage-gaps: FAIL — no tests detected in ${scopes.join(", ")}. Add at least one targeted test or record an explicit waiver.`
  );
  process.exit(1);
}

console.log(
  `validator-coverage-gaps: PASS — tests detected (${hits
    .map((h) => `${h.label}:${h.lines}`)
    .join(", ")}).`
);
