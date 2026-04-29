#!/usr/bin/env node

import { fileURLToPath } from "node:url";
import path from "node:path";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import {
  ensureArtifactRootStructure,
  evaluateArtifactHygiene,
} from "../scripts/lib/artifact-hygiene-lib.mjs";
import { REPO_ROOT, normalizePath } from "../scripts/lib/runtime-paths.mjs";

registerFailCaptureHook("artifact-root-preflight-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("artifact-root-preflight-check.mjs", message, { role: "SHARED", details });
}

export function buildArtifactRootPreflightResult({
  repoRoot = REPO_ROOT,
  artifactRootOverride = "",
  wpId = "",
  repoRoots = null,
} = {}) {
  const artifactRootAbs = ensureArtifactRootStructure(repoRoot, artifactRootOverride);
  const evaluation = evaluateArtifactHygiene({
    repoRoot,
    repoRoots: Array.isArray(repoRoots) ? repoRoots : undefined,
    artifactRootAbs,
  });
  const blockingIssues = evaluation.blockingIssues || [];
  return {
    ok: blockingIssues.length === 0,
    wp_id: String(wpId || "").trim() || null,
    artifact_root_abs: artifactRootAbs,
    failure_class: blockingIssues.length > 0 ? "ENVIRONMENT_BLOCKER" : "NONE",
    revalidation_required: false,
    product_proof_preserved: true,
    blocking_issues: blockingIssues,
    summary: blockingIssues.length === 0
      ? "Artifact root preflight passed; canonical artifact root exists and no blocking local build residue was found."
      : `Artifact root preflight found ${blockingIssues.length} environment blocker(s).`,
  };
}

export function formatArtifactRootPreflightResult(result = {}) {
  const lines = [
    `[ARTIFACT_ROOT_PREFLIGHT] ${result.ok ? "PASS" : "FAIL"}: ${result.summary || ""}`,
    `  - artifact_root=${normalizePath(result.artifact_root_abs || "")}`,
    `  - failure_class=${result.failure_class || "NONE"}`,
    `  - revalidation_required=${result.revalidation_required ? "YES" : "NO"}`,
    `  - product_proof_preserved=${result.product_proof_preserved ? "YES" : "NO"}`,
  ];
  for (const issue of result.blocking_issues || []) {
    lines.push(`  - ENVIRONMENT_BLOCKER: ${issue}`);
  }
  return `${lines.join("\n")}\n`;
}

function main() {
  const args = process.argv.slice(2);
  const wpId = args.find((arg) => /^WP-/.test(String(arg || "").trim())) || "";
  const result = buildArtifactRootPreflightResult({ wpId });
  const rendered = formatArtifactRootPreflightResult(result);
  if (result.ok) {
    process.stdout.write(rendered);
    return;
  }
  fail("Artifact root preflight found environment blockers", rendered.trimEnd().split(/\r?\n/));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
