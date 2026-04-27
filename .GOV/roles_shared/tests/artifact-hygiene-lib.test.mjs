import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import {
  ARTIFACT_RETENTION_MANIFEST_SCHEMA,
  ARTIFACT_RETENTION_MANIFEST_DIR_SEGMENTS,
  buildArtifactRetentionManifest,
  CANONICAL_ARTIFACT_DIRS,
  cleanupArtifactResidue,
  ensureArtifactRootStructure,
  evaluateArtifactHygiene,
  writeArtifactRetentionManifest,
} from "../scripts/lib/artifact-hygiene-lib.mjs";

function makeTempRoot(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function removeTree(absPath) {
  fs.rmSync(absPath, { recursive: true, force: true });
}

function writeCargoConfig(repoRoot, targetDirRel) {
  const cargoDir = path.join(repoRoot, ".cargo");
  fs.mkdirSync(cargoDir, { recursive: true });
  fs.writeFileSync(
    path.join(cargoDir, "config.toml"),
    `[build]\ntarget-dir = "${targetDirRel}"\n`,
    "utf8",
  );
}

test("ensureArtifactRootStructure creates the canonical artifact folders", () => {
  const repoRoot = makeTempRoot("handshake-artifact-root-");
  try {
    const artifactRootAbs = ensureArtifactRootStructure(repoRoot);
    for (const dirName of CANONICAL_ARTIFACT_DIRS) {
      assert.equal(fs.existsSync(path.join(artifactRootAbs, dirName)), true, `${dirName} should exist`);
    }
  } finally {
    removeTree(repoRoot);
  }
});

test("evaluateArtifactHygiene detects repo-local target dirs and stale noncanonical external dirs", () => {
  const workspaceRoot = makeTempRoot("handshake-artifact-eval-");
  const repoRoot = path.join(workspaceRoot, "repo");
  const mainRoot = path.join(workspaceRoot, "handshake_main");
  fs.mkdirSync(path.join(repoRoot, "src", "backend", "handshake_core", "target"), { recursive: true });
  fs.mkdirSync(mainRoot, { recursive: true });

  try {
    const artifactRootAbs = ensureArtifactRootStructure(repoRoot);
    fs.mkdirSync(path.join(artifactRootAbs, "validator_wp1_target"), { recursive: true });
    fs.mkdirSync(path.join(artifactRootAbs, "handshake-cargo-target-release"), { recursive: true });
    const staleDate = new Date(Date.now() - (5 * 60 * 1000));
    fs.utimesSync(path.join(artifactRootAbs, "validator_wp1_target"), staleDate, staleDate);
    writeCargoConfig(mainRoot, "../Handshake_Artifacts/handshake-cargo-target");

    const evaluation = evaluateArtifactHygiene({
      repoRoot,
      repoRoots: [repoRoot, mainRoot],
      artifactRootAbs,
      staleThresholdMs: 60 * 1000,
    });

    assert.equal(evaluation.repoLocalForbiddenDirs.length, 1);
    assert.match(evaluation.repoLocalForbiddenDirs[0].repoRelativePath, /src\/backend\/handshake_core\/target/i);
    assert.equal(evaluation.reclaimableExternalDirs.some((entry) => entry.dirName === "validator_wp1_target"), true);
    assert.equal(evaluation.blockingExternalDirs.some((entry) => entry.dirName === "handshake-cargo-target-release"), true);
  } finally {
    removeTree(workspaceRoot);
  }
});

test("evaluateArtifactHygiene blocks noncanonical sibling artifact roots", () => {
  const workspaceRoot = makeTempRoot("handshake-artifact-sibling-");
  const repoRoot = path.join(workspaceRoot, "repo");
  fs.mkdirSync(repoRoot, { recursive: true });

  try {
    const artifactRootAbs = ensureArtifactRootStructure(repoRoot);
    const driftRootAbs = path.join(workspaceRoot, "Handshake Artifacts");
    fs.mkdirSync(path.join(driftRootAbs, "handshake-cargo-target"), { recursive: true });

    const evaluation = evaluateArtifactHygiene({
      repoRoot,
      repoRoots: [repoRoot],
      artifactRootAbs,
    });
    const summary = cleanupArtifactResidue(evaluation);

    assert.equal(evaluation.siblingArtifactRootDrift.length, 1);
    assert.equal(evaluation.siblingArtifactRootDrift[0].dirName, "Handshake Artifacts");
    assert.equal(evaluation.blockingSiblingArtifactRoots.length, 1);
    assert.equal(evaluation.blockingIssues.some((issue) => issue.includes("Handshake Artifacts")), true);
    assert.equal(summary.removedExternalDirs.includes(driftRootAbs), false);
    assert.equal(fs.existsSync(driftRootAbs), true);
  } finally {
    removeTree(workspaceRoot);
  }
});

test("evaluateArtifactHygiene requires canonical Cargo target config for product checkouts", () => {
  const workspaceRoot = makeTempRoot("handshake-artifact-cargo-required-");
  const repoRoot = path.join(workspaceRoot, "repo");
  const crateRoot = path.join(repoRoot, "src", "backend", "handshake_core");
  fs.mkdirSync(crateRoot, { recursive: true });
  fs.writeFileSync(path.join(crateRoot, "Cargo.toml"), "[package]\nname = \"handshake_core\"\nversion = \"0.0.0\"\n", "utf8");

  try {
    const artifactRootAbs = ensureArtifactRootStructure(repoRoot);
    const evaluation = evaluateArtifactHygiene({
      repoRoot,
      repoRoots: [repoRoot],
      artifactRootAbs,
    });

    assert.equal(evaluation.cargoTargetConfigs.length, 1);
    assert.equal(evaluation.cargoTargetConfigs[0].exists, false);
    assert.equal(evaluation.cargoTargetConfigs[0].requiresCanonicalTargetConfig, true);
    assert.equal(evaluation.blockingIssues.some((issue) => issue.includes("missing required cargo target-dir config")), true);
  } finally {
    removeTree(workspaceRoot);
  }
});

test("cleanupArtifactResidue removes repo-local target dirs and reclaimable external dirs only", () => {
  const workspaceRoot = makeTempRoot("handshake-artifact-clean-");
  const repoRoot = path.join(workspaceRoot, "repo");
  fs.mkdirSync(path.join(repoRoot, "crate", "target"), { recursive: true });

  try {
    const artifactRootAbs = ensureArtifactRootStructure(repoRoot);
    const staleResidue = path.join(artifactRootAbs, "intval-wp1-boundary-target");
    const unknownResidue = path.join(artifactRootAbs, "manual-keepsake");
    fs.mkdirSync(staleResidue, { recursive: true });
    fs.mkdirSync(unknownResidue, { recursive: true });
    const staleDate = new Date(Date.now() - (5 * 60 * 1000));
    fs.utimesSync(staleResidue, staleDate, staleDate);

    const evaluation = evaluateArtifactHygiene({
      repoRoot,
      repoRoots: [repoRoot],
      artifactRootAbs,
      staleThresholdMs: 60 * 1000,
    });
    const summary = cleanupArtifactResidue(evaluation);

    assert.equal(summary.errors.length, 0);
    assert.equal(fs.existsSync(path.join(repoRoot, "crate", "target")), false);
    assert.equal(fs.existsSync(staleResidue), false);
    assert.equal(fs.existsSync(unknownResidue), true);
  } finally {
    removeTree(workspaceRoot);
  }
});

test("artifact retention manifests record retained policy surfaces and write under handshake-tool", () => {
  const workspaceRoot = makeTempRoot("handshake-artifact-retention-");
  const repoRoot = path.join(workspaceRoot, "repo");
  fs.mkdirSync(path.join(repoRoot, "crate", "target"), { recursive: true });

  try {
    const artifactRootAbs = ensureArtifactRootStructure(repoRoot);
    const staleResidue = path.join(artifactRootAbs, "validator_wp1_target");
    fs.mkdirSync(staleResidue, { recursive: true });
    const staleDate = new Date(Date.now() - (5 * 60 * 1000));
    fs.utimesSync(staleResidue, staleDate, staleDate);

    const before = evaluateArtifactHygiene({
      repoRoot,
      repoRoots: [repoRoot],
      artifactRootAbs,
      staleThresholdMs: 60 * 1000,
    });
    const cleanupSummary = cleanupArtifactResidue(before);
    const after = evaluateArtifactHygiene({
      repoRoot,
      repoRoots: [repoRoot],
      artifactRootAbs,
      staleThresholdMs: 60 * 1000,
    });
    const manifest = buildArtifactRetentionManifest({
      repoRoot,
      wpId: "WP-TEST-ARTIFACTS-v1",
      lifecycleScope: "INTEGRATION_VALIDATOR_CLOSEOUT",
      closeoutMode: "CONTAINED_IN_MAIN",
      artifactEvaluationBeforeCleanup: before,
      artifactCleanupSummary: cleanupSummary,
      artifactEvaluationAfterCleanup: after,
    });
    const written = writeArtifactRetentionManifest(manifest, { artifactRootAbs });

    assert.equal(manifest.schema_version, ARTIFACT_RETENTION_MANIFEST_SCHEMA);
    assert.equal(manifest.removed_repo_local_dirs.length, 1);
    assert.equal(manifest.removed_external_dirs.length, 1);
    assert.equal(manifest.retained_canonical_dirs.length, CANONICAL_ARTIFACT_DIRS.length);
    assert.equal(Array.isArray(manifest.retained_sibling_artifact_roots), true);
    assert.equal(
      written.manifestRelPath.startsWith(ARTIFACT_RETENTION_MANIFEST_DIR_SEGMENTS.join("/")),
      true,
    );
    assert.equal(fs.existsSync(written.manifestAbsPath), true);
  } finally {
    removeTree(workspaceRoot);
  }
});
