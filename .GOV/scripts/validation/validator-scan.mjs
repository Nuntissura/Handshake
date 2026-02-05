#!/usr/bin/env node
/**
 * Validator scan: forbidden patterns and placeholder text in backend and frontend sources.
 * Exits non-zero if any finding is detected.
 */
import { execSync } from "node:child_process";

const targets = ["src/backend/handshake_core/src", "app/src"];
const GLOB_RS = '--glob "*.rs"';

const forbidden = [
  "\\\\bsplit_whitespace\\\\(\\\\)",
  "\\\\bunwrap\\\\(\\\\)",
  "expect\\(",
  "todo!",
  "unimplemented!",
  "dbg!",
  "println!",
  "eprintln!",
  "panic!",
];

const placeholder = ["Mock", "Stub", "placeholder", "hollow"];
const placeholderPathExcludes = ["governance_pack.rs:"];

function runRg(pattern, paths = targets, extraArgs = "") {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${paths.join(
    " "
  )} ${GLOB_RS} ${extraArgs}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
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
  let out = runRg(pat);
  if (out) {
    out = out
      .split("\n")
      .filter((line) => !placeholderPathExcludes.some((ex) => line.includes(ex)))
      .join("\n")
      .trim();
  }
  if (out) {
    findings.push(`PLACEHOLDER/MOCK "${pat}":\n${out}`);
  }
}

if (findings.length > 0) {
  console.error("validator-scan: FAIL - findings detected");
  findings.forEach((f) => {
    console.error("----");
    console.error(f);
  });
  process.exit(1);
}

console.log("validator-scan: PASS - no forbidden patterns detected in backend sources.");
