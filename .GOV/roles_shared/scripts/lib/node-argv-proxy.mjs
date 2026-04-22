#!/usr/bin/env node

import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

function usage() {
  console.error('Usage: node-argv-proxy.mjs <target-script> [base-args ...] --raw-flags "<flags string>"');
  process.exit(1);
}

export function splitRawFlags(rawFlags) {
  const text = String(rawFlags || "").trim();
  if (!text) return [];
  const tokens = [];
  let current = "";
  let quote = "";
  let tokenStarted = false;

  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    const nextChar = text[index + 1] || "";

    if (quote) {
      if (char === quote) {
        quote = "";
        continue;
      }
      if (quote === '"' && char === "\\" && (nextChar === '"' || nextChar === "\\")) {
        current += nextChar;
        index += 1;
        continue;
      }
      current += char;
      continue;
    }

    if (/\s/.test(char)) {
      if (tokenStarted) {
        tokens.push(current);
        current = "";
        tokenStarted = false;
      }
      continue;
    }

    if ((char === '"' || char === "'") && (current === "" || current.endsWith("="))) {
      quote = char;
      tokenStarted = true;
      continue;
    }

    if (char === "\\" && (nextChar === '"' || nextChar === "'" || nextChar === "\\" || /\s/.test(nextChar))) {
      current += nextChar;
      tokenStarted = true;
      index += 1;
      continue;
    }

    current += char;
    tokenStarted = true;
  }

  if (quote) {
    throw new Error(`Unterminated ${quote} quote in --raw-flags payload.`);
  }
  if (tokenStarted) {
    tokens.push(current);
  }
  return tokens;
}

export function buildForwardedArgv(argv = []) {
  const separatorIndex = argv.indexOf("--raw-flags");
  if (separatorIndex === -1) usage();

  const [targetScript, ...baseArgs] = argv.slice(0, separatorIndex);
  if (!targetScript) usage();

  // PowerShell drops empty-string args for native processes, so `--raw-flags ""`
  // can arrive as a trailing `--raw-flags` with no payload.
  const rawFlags = argv[separatorIndex + 1] || "";
  return {
    targetScript,
    forwardedArgs: [...baseArgs, ...splitRawFlags(rawFlags)],
  };
}

function isDirectExecution() {
  const entry = process.argv[1];
  if (!entry) return false;
  return path.resolve(entry) === fileURLToPath(import.meta.url);
}

function main(argv = process.argv.slice(2)) {
  const { targetScript, forwardedArgs } = buildForwardedArgv(argv);
  const result = spawnSync(process.execPath, [targetScript, ...forwardedArgs], {
    cwd: process.cwd(),
    stdio: "inherit",
    shell: false,
  });
  process.exit(result.status ?? 1);
}

if (isDirectExecution()) {
  main();
}
