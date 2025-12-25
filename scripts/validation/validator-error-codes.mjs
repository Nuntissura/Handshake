#!/usr/bin/env node
/**
 * Error/trace determinism audit:
 * - Flags stringly errors in production paths
 * - Flags unseeded randomness/time sources in production paths
 *
 * Exits non-zero on findings.
 */
import { execSync } from "node:child_process";

const targets = ["src/backend/handshake_core/src"];
const exclude = '--glob "!**/tests/**" --glob "!**/*test*/*" --glob "!**/*test*.*"';

const stringErrorPatterns = [
  'Err\\(\\s*"', // Err("msg")
  "Err\\(\\s*String::from",
  "Err\\(\\s*format!",
  'map_err\\(\\|.*\\|\\s*"', // map_err(|_| "msg")
  "map_err\\(\\|.*\\|\\s*String::from",
  "map_err\\(\\|.*\\|\\s*format!",
  "\\.to_string\\(\\)\\s*\\)?\\s*[,)]\\s*$", // loose to_string in returns
  "anyhow!\\(",
  "bail!\\(",
];

const nondeterminismPatterns = [
  "rand::",
  "thread_rng",
  "rand\\(",
  "Instant::now\\(",
  "SystemTime::now\\(",
];

function runRg(pattern, label) {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${targets.join(
    " "
  )} ${exclude}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    console.error(`validator-error-codes: ${label} scan failed: ${err.message}`);
    process.exit(1);
  }
}

const findings = [];

for (const pat of stringErrorPatterns) {
  const out = runRg(pat, "string-error");
  if (out) {
    findings.push(`STRING_ERROR pattern "${pat}":\n${out}`);
  }
}

for (const pat of nondeterminismPatterns) {
  const out = runRg(pat, "determinism");
  if (out) {
    findings.push(`NONDETERMINISM pattern "${pat}":\n${out}`);
  }
}

// Require at least some HSK-#### codes in the tree to ensure convention is present; warn if missing.
const codesOut = runRg("HSK-[0-9]{3,}", "hsk-codes");
if (!codesOut) {
  findings.push("WARNING: No HSK-#### error codes found in targets; ensure typed errors include stable codes.");
}

if (findings.length > 0) {
  console.error("validator-error-codes: FAIL/WARN findings detected");
  findings.forEach((f) => {
    console.error("----");
    console.error(f);
  });
  process.exit(1);
}

console.log("validator-error-codes: PASS — no stringly errors or nondeterminism patterns detected.");
