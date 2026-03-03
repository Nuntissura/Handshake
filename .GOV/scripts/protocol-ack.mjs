#!/usr/bin/env node
/**
 * Protocol Ack Helper
 * Prints the first non-empty line from each provided file path.
 *
 * Intended use: role startup prompts can combine "I read the docs" proof
 * with the hard-gate/preflight output in a single message (anti-babysit).
 */

import fs from "node:fs";

function firstNonEmptyLine(raw) {
  const lines = raw.split(/\r?\n/);
  for (const line of lines) {
    if (line.trim() !== "") return line;
  }
  return "";
}

const files = process.argv.slice(2);
if (files.length === 0) {
  console.error("Usage: node .GOV/scripts/protocol-ack.mjs <file> [file...]");
  process.exit(1);
}

let missing = false;
console.log("PROTOCOL_ACK (first non-empty line from each file read)");
for (const file of files) {
  if (!fs.existsSync(file)) {
    missing = true;
    console.log(`- ${file}: <MISSING>`);
    continue;
  }
  const raw = fs.readFileSync(file, "utf8");
  const line = firstNonEmptyLine(raw);
  console.log(`- ${file}: ${line}`);
}

if (missing) process.exit(2);

