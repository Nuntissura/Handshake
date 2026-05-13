#!/usr/bin/env node
/**
 * Governance reference resolver (SSoT).
 *
 * Single source of truth:
 * - .GOV/spec/SPEC_CURRENT.md JSON -> governance_reference.path
 *
 * This helper is used by CI / hooks to avoid hardcoding legacy filenames (e.g. Codex v0.8).
 */

import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";
import { resolveGovernanceReferenceFromSpecCurrentAtRepo } from "../scripts/lib/spec-current-lib.mjs";

const SPEC_CURRENT_REL = path.join(GOV_ROOT_REPO_REL, "spec", "SPEC_CURRENT.md");
const FILE_RELATIVE_REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");

function tryGitRepoRoot() {
  try {
    return execFileSync("git", ["-C", FILE_RELATIVE_REPO_ROOT, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "";
  }
}

export function resolveGovernanceReference(options = {}) {
  const repoRoot = options.repoRoot || tryGitRepoRoot() || FILE_RELATIVE_REPO_ROOT;
  return resolveGovernanceReferenceFromSpecCurrentAtRepo(repoRoot, {
    specCurrentPath: options.specCurrentPath || SPEC_CURRENT_REL,
  });
}

function printUsageAndExit() {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles_shared/checks/governance-reference.mjs [--print-file|--print-path|--json]`);
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
