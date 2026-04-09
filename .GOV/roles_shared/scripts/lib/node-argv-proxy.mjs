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
  return text.split(/\s+/).filter(Boolean);
}

export function buildForwardedArgv(argv = []) {
  const separatorIndex = argv.indexOf("--raw-flags");
  if (separatorIndex === -1 || separatorIndex === argv.length - 1) usage();

  const [targetScript, ...baseArgs] = argv.slice(0, separatorIndex);
  if (!targetScript) usage();

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
