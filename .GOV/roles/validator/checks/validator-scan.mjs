#!/usr/bin/env node
/**
 * Validator scan: forbidden patterns and placeholder text in backend and frontend sources.
 * Exits non-zero if any finding is detected.
 */
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

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
// Note: avoid scanning for the substring "placeholder" in Rust, because it's a legitimate domain term
// in `spec_router/*` and would create perpetual false-positives.
const placeholderRust = ["Mock", "Stub", "hollow"];
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

const cfgTestStartLineCache = new Map();

function getCfgTestStartLine(filePath) {
  const normalized = path.normalize(filePath);
  if (cfgTestStartLineCache.has(normalized)) {
    return cfgTestStartLineCache.get(normalized);
  }
  try {
    const text = fs.readFileSync(normalized, "utf8");
    const lines = text.split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].includes("#[cfg(test)]")) {
        const start = i + 1; // 1-based for `rg --line-number`
        cfgTestStartLineCache.set(normalized, start);
        return start;
      }
    }
  } catch {
    // If the file can't be read, don't filter anything.
  }
  cfgTestStartLineCache.set(normalized, null);
  return null;
}

function filterOutCfgTestMatches(rgOut) {
  if (!rgOut) return "";

  const kept = [];
  for (const line of rgOut.split("\n")) {
    const firstColon = line.indexOf(":");
    const secondColon = firstColon === -1 ? -1 : line.indexOf(":", firstColon + 1);
    if (firstColon === -1 || secondColon === -1) {
      kept.push(line);
      continue;
    }
    const rawPath = line.slice(0, firstColon);
    const lineNoStr = line.slice(firstColon + 1, secondColon);
    const lineNo = Number.parseInt(lineNoStr, 10);
    if (!Number.isFinite(lineNo)) {
      kept.push(line);
      continue;
    }

    // Heuristic: unit tests live in `#[cfg(test)]` regions; we filter those matches so the scan
    // enforces production-code hygiene without forcing test code to avoid `expect/unwrap/panic`.
    const cfgStart = getCfgTestStartLine(rawPath);
    if (cfgStart && lineNo >= cfgStart) {
      continue;
    }
    kept.push(line);
  }

  return kept.join("\n").trim();
}

const findings = [];

for (const pat of forbiddenRust) {
  let out = runRg(pat, rustTargets, GLOB_RS);
  out = filterOutCfgTestMatches(out);
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
  out = filterOutCfgTestMatches(out);
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
