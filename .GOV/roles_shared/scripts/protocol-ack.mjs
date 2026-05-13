#!/usr/bin/env node
/**
 * Protocol Ack Helper
 * Prints the first non-empty line from each provided file path.
 *
 * Intended use: role startup prompts can combine "I read the docs" proof
 * with the hard-gate/preflight output in a single message (anti-babysit).
 */

import fs from "node:fs";
import path from "node:path";
import {
  formatProtectedWorktreeResolutionDiagnostics,
  resolveProtectedWorktree,
} from "./topology/git-topology-lib.mjs";

function firstNonEmptyLine(raw) {
  const lines = raw.split(/\r?\n/);
  for (const line of lines) {
    if (line.trim() !== "") return line;
  }
  return "";
}

const files = process.argv.slice(2);
let full = false;
const paths = [];
for (const arg of files) {
  if (arg === "--full") {
    full = true;
    continue;
  }
  if (arg.startsWith("--")) {
    console.error(`Unknown flag: ${arg}`);
    process.exit(2);
  }
  paths.push(arg);
}

if (paths.length === 0) {
  console.error("Usage: node .GOV/roles_shared/scripts/protocol-ack.mjs [--full] <file> [file...]");
  process.exit(1);
}

let missing = false;
console.log(`PROTOCOL_ACK (${full ? "full content" : "first non-empty line from each file"})`);
for (const file of paths) {
  let resolvedFile = file;
  const normalized = String(file || "").replace(/\\/g, "/");
  if (!fs.existsSync(resolvedFile) && /(^|\/)handshake_main\/AGENTS\.md$/i.test(normalized)) {
    const mainResolution = resolveProtectedWorktree("handshake_main");
    const candidate = mainResolution.absDir ? path.join(mainResolution.absDir, "AGENTS.md") : "";
    if (candidate && fs.existsSync(candidate)) {
      resolvedFile = candidate;
    } else {
      console.log(`- ${file}: <MISSING>`);
      for (const line of formatProtectedWorktreeResolutionDiagnostics(mainResolution)) {
        console.log(`  ${line}`);
      }
      missing = true;
      continue;
    }
  }

  if (!fs.existsSync(resolvedFile)) {
    missing = true;
    console.log(`- ${file}: <MISSING>`);
    continue;
  }
  const raw = fs.readFileSync(resolvedFile, "utf8");
  const label = resolvedFile === file ? file : `${file} -> ${resolvedFile}`;
  if (full) {
    const normalizedFile = resolvedFile === file ? file : `${file} -> ${resolvedFile}`;
    console.log(`AUTHORITY_FILE_BEGIN path=${normalizedFile}`);
    console.log(raw);
    console.log(`AUTHORITY_FILE_END path=${normalizedFile}`);
    console.log("");
  } else {
    const line = firstNonEmptyLine(raw);
    console.log(`- ${label}: ${line}`);
  }
}

if (missing) process.exit(2);
