#!/usr/bin/env node
/**
 * Validator scan: forbidden patterns, mocks/placeholders, and boundary greps.
 * Exits non-zero if any finding is detected.
 */
import { execSync } from "node:child_process";

const targets = ["src/backend/handshake_core/src", "app/src"];

const forbidden = [
  "split_whitespace",
  "unwrap",
  "expect\\(",
  "todo!",
  "unimplemented!",
  "dbg!",
  "println!",
  "eprintln!",
  "panic!",
];

const placeholder = ["Mock", "Stub", "placeholder", "hollow"];

function runRg(pattern, extraArgs = "") {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${targets.join(
    " "
  )} ${extraArgs}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    // rg exits 1 on no matches; ignore that case
    if (err.status === 1) return "";
    throw err;
  }
}

const findings = [];

for (const pat of forbidden) {
  const out = runRg(pat);
  if (out) {
    findings.push(`FORBIDDEN_PATTERN "${pat}":\n${out}`);
  }
}

for (const pat of placeholder) {
  const out = runRg(pat);
  if (out) {
    findings.push(`PLACEHOLDER/MOCK "${pat}":\n${out}`);
  }
}

if (findings.length > 0) {
  console.error("validator-scan: FAIL — findings detected");
  findings.forEach((f) => {
    console.error("----");
    console.error(f);
  });
  process.exit(1);
}

console.log("validator-scan: PASS — no forbidden patterns detected in target paths.");
