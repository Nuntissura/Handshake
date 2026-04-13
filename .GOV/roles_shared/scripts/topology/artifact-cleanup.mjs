#!/usr/bin/env node

import {
  REPO_ROOT,
  normalizePath,
} from "../lib/runtime-paths.mjs";
import {
  buildArtifactRetentionManifest,
  cleanupArtifactResidue,
  ensureArtifactRootStructure,
  evaluateArtifactHygiene,
  writeArtifactRetentionManifest,
} from "../lib/artifact-hygiene-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("artifact-cleanup.mjs", { role: "SHARED" });

const dryRun = process.argv.slice(2).includes("--dry-run");

function fail(message, details = []) {
  failWithMemory("artifact-cleanup.mjs", message, { role: "SHARED", details });
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
const manifest = buildArtifactRetentionManifest({
  repoRoot: REPO_ROOT,
  lifecycleScope: dryRun ? "MANUAL_DRY_RUN" : "MANUAL_CLEANUP",
  dryRun,
  artifactEvaluationBeforeCleanup: evaluation,
  artifactCleanupSummary: summary,
  artifactEvaluationAfterCleanup: post,
});
const manifestWrite = writeArtifactRetentionManifest(manifest, {
  artifactRootAbs: post.artifactRootAbs,
});

console.log(`[ARTIFACT_CLEANUP] PASS: artifact cleanup ${dryRun ? "simulated" : "completed"}`);
console.log(`  artifact_root=${normalizePath(post.artifactRootAbs)}`);
console.log(`  removed_repo_local_dirs=${summary.removedRepoLocalDirs.map((entry) => normalizePath(entry)).join(", ") || "<none>"}`);
console.log(`  removed_external_dirs=${summary.removedExternalDirs.map((entry) => normalizePath(entry)).join(", ") || "<none>"}`);
console.log(`  retention_manifest=${normalizePath(manifestWrite.manifestAbsPath)}`);
