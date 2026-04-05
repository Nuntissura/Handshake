#!/usr/bin/env node

import {
  REPO_ROOT,
  normalizePath,
} from "../lib/runtime-paths.mjs";
import {
  cleanupArtifactResidue,
  ensureArtifactRootStructure,
  evaluateArtifactHygiene,
} from "../lib/artifact-hygiene-lib.mjs";

const dryRun = process.argv.slice(2).includes("--dry-run");

function fail(message, details = []) {
  console.error(`[ARTIFACT_CLEANUP] FAIL: ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(1);
}

ensureArtifactRootStructure(REPO_ROOT);
const evaluation = evaluateArtifactHygiene({ repoRoot: REPO_ROOT });
const summary = cleanupArtifactResidue(evaluation, { dryRun });
if (summary.errors.length > 0) {
  fail("artifact cleanup encountered errors", summary.errors);
}

const post = evaluateArtifactHygiene({ repoRoot: REPO_ROOT });
if (post.blockingIssues.length > 0) {
  fail("blocking artifact hygiene violations remain after cleanup", post.blockingIssues);
}

console.log(`[ARTIFACT_CLEANUP] PASS: artifact cleanup ${dryRun ? "simulated" : "completed"}`);
console.log(`  artifact_root=${normalizePath(post.artifactRootAbs)}`);
console.log(`  removed_repo_local_dirs=${summary.removedRepoLocalDirs.map((entry) => normalizePath(entry)).join(", ") || "<none>"}`);
console.log(`  removed_external_dirs=${summary.removedExternalDirs.map((entry) => normalizePath(entry)).join(", ") || "<none>"}`);
