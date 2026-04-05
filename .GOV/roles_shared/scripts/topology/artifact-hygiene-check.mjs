#!/usr/bin/env node

import {
  REPO_ROOT,
  normalizePath,
} from "../lib/runtime-paths.mjs";
import {
  evaluateArtifactHygiene,
} from "../lib/artifact-hygiene-lib.mjs";

function fail(message, details = []) {
  console.error(`[ARTIFACT_HYGIENE_CHECK] FAIL: ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(1);
}

function pass(message, details = []) {
  console.log(`[ARTIFACT_HYGIENE_CHECK] PASS: ${message}`);
  for (const detail of details) console.log(`  ${detail}`);
}

const evaluation = evaluateArtifactHygiene({ repoRoot: REPO_ROOT });
if (evaluation.blockingIssues.length > 0) {
  fail("artifact hygiene violations detected", [
    ...evaluation.blockingIssues,
    `reclaimable_external_dirs=${evaluation.reclaimableExternalDirs.map((entry) => entry.dirName).join(", ") || "<none>"}`,
  ]);
}

pass("artifact root and repo-local hygiene are coherent", [
  `artifact_root=${normalizePath(evaluation.artifactRootAbs)}`,
  `repo_roots=${evaluation.repoRoots.map((entry) => normalizePath(entry)).join(", ")}`,
  `reclaimable_external_dirs=${evaluation.reclaimableExternalDirs.map((entry) => entry.dirName).join(", ") || "<none>"}`,
]);
