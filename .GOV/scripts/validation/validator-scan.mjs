#!/usr/bin/env node
/**
 * Validator scan: forbidden patterns and placeholder text in backend and frontend sources.
 * Exits non-zero if any finding is detected.
 */
import { execSync } from "node:child_process";

const rustTargets = ["src/backend/handshake_core/src"];
const frontendTargets = ["app/src"];
const GLOB_RS = ['--glob "*.rs"'];
const GLOB_FRONTEND = ['--glob "*.ts"', '--glob "*.tsx"', '--glob "*.js"', '--glob "*.jsx"'];

const forbiddenRust = [
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

// Rust placeholders are stricter than frontend because frontend legitimately uses `placeholder=\"...\"`.
const placeholderRust = ["Mock", "Stub", "placeholder", "hollow"];
const forbiddenFrontend = ["debugger", "console\\\\.log", "(it|describe)\\\\.only"];
const placeholderFrontend = [];
const placeholderPathExcludes = ["governance_pack.rs:"];

function runRg(pattern, paths, globArgs = [], extraArgs = "") {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${paths.join(
    " "
  )} ${globArgs.join(" ")} ${extraArgs}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    throw err;
  }
}

const findings = [];

for (const pat of forbiddenRust) {
  const out = runRg(pat, rustTargets, GLOB_RS);
  if (out) {
    findings.push(`FORBIDDEN_PATTERN (rust) "${pat}":\n${out}`);
  }
}

for (const pat of forbiddenFrontend) {
  const out = runRg(pat, frontendTargets, GLOB_FRONTEND);
  if (out) {
    findings.push(`FORBIDDEN_PATTERN (frontend) "${pat}":\n${out}`);
  }
}

for (const pat of placeholderRust) {
  let out = runRg(pat, rustTargets, GLOB_RS);
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

for (const pat of placeholderFrontend) {
  const out = runRg(pat, frontendTargets, GLOB_FRONTEND);
  if (out) {
    findings.push(`PLACEHOLDER/MOCK (frontend) "${pat}":\n${out}`);
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

console.log("validator-scan: PASS - no forbidden patterns detected in backend/frontend sources.");
