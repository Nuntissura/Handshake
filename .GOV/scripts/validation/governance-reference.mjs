#!/usr/bin/env node
/**
 * Governance reference resolver (SSoT).
 *
 * Single source of truth:
 * - .GOV/roles_shared/SPEC_CURRENT.md -> "The current authoritative Governance Reference is:" -> **<codex filename>**
 *
 * This helper is used by CI / hooks to avoid hardcoding legacy filenames (e.g. Codex v0.8).
 */

import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const SPEC_CURRENT_REL = path.join(".GOV", "roles_shared", "SPEC_CURRENT.md");

function tryGitRepoRoot() {
  try {
    return execSync("git rev-parse --show-toplevel", { encoding: "utf8" }).trim();
  } catch {
    return "";
  }
}

function extractBoldFilenameFromSection({ specCurrentText, sectionMarkerRe, specCurrentPath }) {
  const lines = specCurrentText.split(/\r?\n/);
  const markerIdx = lines.findIndex((l) => sectionMarkerRe.test(l));
  if (markerIdx === -1) {
    throw new Error(
      `Could not find governance reference section marker in ${specCurrentPath} (expected line matching ${sectionMarkerRe}).`
    );
  }

  for (let i = markerIdx + 1; i < Math.min(lines.length, markerIdx + 30); i += 1) {
    const line = (lines[i] || "").trim();
    if (!line) continue;
    const m = line.match(/\*\*(.+?)\*\*/);
    if (m && m[1]) return m[1].trim();
  }

  throw new Error(
    `Could not parse governance reference filename from ${specCurrentPath} (expected **<filename>** line after marker).`
  );
}

export function resolveGovernanceReference(options = {}) {
  const repoRoot = options.repoRoot || tryGitRepoRoot() || process.cwd();
  let specCurrentPathAbs = path.resolve(repoRoot, options.specCurrentPath || SPEC_CURRENT_REL);
  if (!options.specCurrentPath && !fs.existsSync(specCurrentPathAbs)) {
    // Legacy compatibility bundle (should not be treated as governance SSoT).
    const compat = path.resolve(repoRoot, "docs", "SPEC_CURRENT.md");
    if (fs.existsSync(compat)) specCurrentPathAbs = compat;
  }
  const specCurrentText = fs.readFileSync(specCurrentPathAbs, "utf8");

  const codexFilename = extractBoldFilenameFromSection({
    specCurrentText,
    sectionMarkerRe: /the current authoritative governance reference is\s*:/i,
    specCurrentPath: specCurrentPathAbs,
  });

  const codexPathAbs = path.resolve(repoRoot, codexFilename);
  return { codexFilename, codexPathAbs, specCurrentPathAbs };
}

function printUsageAndExit() {
  console.error("Usage: node .GOV/scripts/validation/governance-reference.mjs [--print-file|--print-path|--json]");
  process.exit(2);
}

function main(argv) {
  const args = argv.slice(2);
  const flags = new Set(args.filter((a) => a.startsWith("--")));
  if (flags.size === 0) printUsageAndExit();

  const ref = resolveGovernanceReference();

  if (flags.has("--print-file")) {
    process.stdout.write(`${ref.codexFilename}\n`);
    return;
  }

  if (flags.has("--print-path")) {
    process.stdout.write(`${ref.codexPathAbs}\n`);
    return;
  }

  if (flags.has("--json")) {
    process.stdout.write(`${JSON.stringify(ref)}\n`);
    return;
  }

  printUsageAndExit();
}

const thisFile = fileURLToPath(import.meta.url);
const invokedFile = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedFile && path.resolve(invokedFile) === path.resolve(thisFile)) {
  main(process.argv);
}


