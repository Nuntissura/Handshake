import fs from "node:fs";
import path from "node:path";
import { discoverGitCheckouts } from "../topology/git-topology-lib.mjs";
import { REPO_ROOT, normalizePath } from "./runtime-paths.mjs";

export const HANDSHAKE_ARTIFACT_ROOT_ENV_VAR = "HANDSHAKE_ARTIFACT_ROOT";
export const DEFAULT_ARTIFACT_ROOT_DIRNAME = "Handshake_Artifacts";
export const CANONICAL_CARGO_TARGET_DIRNAME = "handshake-cargo-target";
export const PRODUCT_CARGO_MANIFEST_REPO_REL = normalizePath(path.join("src", "backend", "handshake_core", "Cargo.toml"));
export const CANONICAL_ARTIFACT_DIRS = Object.freeze([
  CANONICAL_CARGO_TARGET_DIRNAME,
  "handshake-product",
  "handshake-test",
  "handshake-tool",
]);
export const ARTIFACT_RETENTION_MANIFEST_SCHEMA = "hsk.artifact_retention_manifest@1";
export const ARTIFACT_RETENTION_POLICY_VERSION = "2026-04-05";
export const ARTIFACT_RETENTION_MANIFEST_DIR_SEGMENTS = Object.freeze([
  "handshake-tool",
  "artifact-retention",
]);

const REPO_SCAN_SKIP_DIRS = new Set([
  ".git",
  "node_modules",
  ".next",
  ".turbo",
  "dist",
  "build",
  "out",
  "gov_runtime",
  DEFAULT_ARTIFACT_ROOT_DIRNAME,
]);
const FORBIDDEN_REPO_LOCAL_DIR_NAMES = new Set(["target"]);
const NONCANONICAL_EPHEMERAL_DIR_RE =
  /(?:^|[-_])(validator|wpval|intval|orchestrator|coder|probe|scratch|tmp)(?:[-_]|$)|(?:^|[-_])wp\d+(?:[-_]|$)|(?:^|[-_])target(?:[-_]|$)|(?:^|[-_]).*target$/i;
const ARTIFACT_CARGO_TARGET_RE = /^\s*target-dir\s*=\s*"([^"]+)"\s*$/mi;
const DEFAULT_STALE_ARTIFACT_AGE_MS = 60 * 1000;

function safeStat(absPath) {
  try {
    return fs.statSync(absPath);
  } catch {
    return null;
  }
}

function normalizeComparablePath(value) {
  return normalizePath(path.resolve(String(value || ""))).toLowerCase();
}

function artifactRootNameFingerprint(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[\s_-]+/g, "");
}

function sanitizeFileSegment(value) {
  return String(value || "")
    .trim()
    .replace(/[^A-Za-z0-9._-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "")
    || "artifact";
}

function directoryChildren(absDir) {
  try {
    return fs.readdirSync(absDir, { withFileTypes: true });
  } catch {
    return [];
  }
}

function ensureWithin(parentAbs, candidateAbs) {
  const parent = normalizeComparablePath(parentAbs);
  const candidate = normalizeComparablePath(candidateAbs);
  return candidate === parent || candidate.startsWith(`${parent}/`);
}

function parseCargoTargetDir(repoRoot) {
  const cargoConfigAbs = path.resolve(repoRoot, ".cargo", "config.toml");
  if (!fs.existsSync(cargoConfigAbs)) {
    return {
      cargoConfigAbs,
      declaredTargetDir: "",
      resolvedTargetDirAbs: "",
      exists: false,
    };
  }
  const content = fs.readFileSync(cargoConfigAbs, "utf8");
  const match = content.match(ARTIFACT_CARGO_TARGET_RE);
  const declaredTargetDir = match ? String(match[1] || "").trim() : "";
  return {
    cargoConfigAbs,
    declaredTargetDir,
    // Cargo config paths here are governed relative to the repo/workspace root, not the .cargo folder.
    resolvedTargetDirAbs: declaredTargetDir ? path.resolve(repoRoot, declaredTargetDir) : "",
    exists: true,
  };
}

function repoRequiresCargoTargetConfig(repoRoot) {
  return fs.existsSync(path.resolve(repoRoot, PRODUCT_CARGO_MANIFEST_REPO_REL));
}

function scanForbiddenRepoLocalDirs(repoRoot) {
  const discovered = [];
  const stack = [path.resolve(repoRoot)];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of directoryChildren(current)) {
      if (!entry.isDirectory()) continue;
      const absPath = path.join(current, entry.name);
      if (REPO_SCAN_SKIP_DIRS.has(entry.name)) continue;
      if (FORBIDDEN_REPO_LOCAL_DIR_NAMES.has(entry.name)) {
        discovered.push({
          kind: "REPO_LOCAL_FORBIDDEN_DIR",
          dirName: entry.name,
          absPath,
          repoRootAbs: path.resolve(repoRoot),
          repoRelativePath: normalizePath(path.relative(repoRoot, absPath)),
        });
        continue;
      }
      stack.push(absPath);
    }
  }
  return discovered.sort((left, right) =>
    left.repoRootAbs.localeCompare(right.repoRootAbs) || left.repoRelativePath.localeCompare(right.repoRelativePath)
  );
}

function scanSiblingArtifactRootDrift(artifactRootAbs) {
  const rootAbs = path.resolve(artifactRootAbs);
  const parentAbs = path.dirname(rootAbs);
  const canonicalDirName = path.basename(rootAbs);
  const canonicalFingerprint = artifactRootNameFingerprint(canonicalDirName);

  return directoryChildren(parentAbs)
    .filter((entry) => entry.isDirectory())
    .map((entry) => {
      const absPath = path.join(parentAbs, entry.name);
      if (normalizeComparablePath(absPath) === normalizeComparablePath(rootAbs)) return null;
      if (artifactRootNameFingerprint(entry.name) !== canonicalFingerprint) return null;
      return {
        kind: "NONCANONICAL_SIBLING_ARTIFACT_ROOT",
        dirName: entry.name,
        absPath,
        canonicalArtifactRootAbs: rootAbs,
        blocking: true,
        reclaimable: false,
        reason: `noncanonical sibling artifact root; use ${canonicalDirName} and remove or quarantine this root after review`,
      };
    })
    .filter(Boolean)
    .sort((left, right) => left.dirName.localeCompare(right.dirName));
}

function classifyExternalArtifactDir({
  artifactRootAbs,
  dirName,
  absPath,
  staleThresholdMs = DEFAULT_STALE_ARTIFACT_AGE_MS,
  nowMs = Date.now(),
} = {}) {
  const stats = safeStat(absPath);
  const ageMs = stats ? Math.max(0, nowMs - stats.mtimeMs) : Number.POSITIVE_INFINITY;
  const canonical = CANONICAL_ARTIFACT_DIRS.includes(dirName);
  if (canonical) {
    return {
      kind: "CANONICAL",
      dirName,
      absPath,
      ageMs,
      reclaimable: false,
      blocking: false,
      reason: "canonical artifact directory",
    };
  }

  const ephemeral = NONCANONICAL_EPHEMERAL_DIR_RE.test(dirName);
  const stale = ageMs >= staleThresholdMs;
  if (ephemeral && stale) {
    return {
      kind: "NONCANONICAL_EPHEMERAL_STALE",
      dirName,
      absPath,
      ageMs,
      reclaimable: true,
      blocking: false,
      reason: `noncanonical stale governed artifact directory under ${normalizePath(path.relative(path.dirname(artifactRootAbs), absPath))}`,
    };
  }
  if (ephemeral) {
    return {
      kind: "NONCANONICAL_EPHEMERAL_RECENT",
      dirName,
      absPath,
      ageMs,
      reclaimable: false,
      blocking: true,
      reason: "noncanonical governed artifact directory is still recent; refuse to auto-delete while it may still be active",
    };
  }
  return {
    kind: "NONCANONICAL_UNKNOWN",
    dirName,
    absPath,
    ageMs,
    reclaimable: false,
    blocking: true,
    reason: "unknown noncanonical artifact directory",
  };
}

function removeDirSafe(absPath, allowedRootAbs) {
  const candidateAbs = path.resolve(absPath);
  if (!ensureWithin(allowedRootAbs, candidateAbs)) {
    throw new Error(`refusing to remove path outside allowed root: ${normalizePath(candidateAbs)}`);
  }
  fs.rmSync(candidateAbs, { recursive: true, force: true });
}

export function resolveArtifactRoot(repoRoot = REPO_ROOT, overrideValue = "") {
  const directValue = String(overrideValue || process.env[HANDSHAKE_ARTIFACT_ROOT_ENV_VAR] || "").trim();
  if (directValue) return path.resolve(directValue);
  return path.resolve(repoRoot, "..", DEFAULT_ARTIFACT_ROOT_DIRNAME);
}

export function defaultArtifactRepoRoots(repoRoot = REPO_ROOT) {
  const normalizedRepoRoot = path.resolve(repoRoot);
  const discovered = discoverGitCheckouts()
    .map((entry) => path.resolve(entry.abs_dir))
    .filter((candidate) => ensureWithin(path.resolve(repoRoot, ".."), candidate));
  const unique = new Map();
  for (const candidate of [normalizedRepoRoot, ...discovered]) {
    const comparable = normalizeComparablePath(candidate);
    if (!unique.has(comparable)) {
      unique.set(comparable, candidate);
    }
  }
  return Array.from(unique.values()).sort();
}

export function ensureArtifactRootStructure(repoRoot = REPO_ROOT, overrideValue = "") {
  const artifactRootAbs = resolveArtifactRoot(repoRoot, overrideValue);
  fs.mkdirSync(artifactRootAbs, { recursive: true });
  for (const dirName of CANONICAL_ARTIFACT_DIRS) {
    fs.mkdirSync(path.join(artifactRootAbs, dirName), { recursive: true });
  }
  fs.mkdirSync(path.join(artifactRootAbs, ...ARTIFACT_RETENTION_MANIFEST_DIR_SEGMENTS), { recursive: true });
  return artifactRootAbs;
}

export function evaluateArtifactHygiene({
  repoRoot = REPO_ROOT,
  repoRoots = defaultArtifactRepoRoots(repoRoot),
  artifactRootAbs = resolveArtifactRoot(repoRoot),
  staleThresholdMs = DEFAULT_STALE_ARTIFACT_AGE_MS,
  nowMs = Date.now(),
} = {}) {
  const rootAbs = path.resolve(artifactRootAbs);
  const externalArtifactDirs = fs.existsSync(rootAbs)
    ? directoryChildren(rootAbs)
      .filter((entry) => entry.isDirectory())
      .map((entry) => classifyExternalArtifactDir({
        artifactRootAbs: rootAbs,
        dirName: entry.name,
        absPath: path.join(rootAbs, entry.name),
        staleThresholdMs,
        nowMs,
      }))
      .sort((left, right) => left.dirName.localeCompare(right.dirName))
    : [];

  const repoLocalForbiddenDirs = repoRoots
    .flatMap((candidateRoot) => scanForbiddenRepoLocalDirs(candidateRoot))
    .sort((left, right) =>
      left.repoRootAbs.localeCompare(right.repoRootAbs) || left.repoRelativePath.localeCompare(right.repoRelativePath)
    );

  const cargoTargetConfigs = repoRoots
    .map((candidateRoot) => {
      const parsed = parseCargoTargetDir(candidateRoot);
      const expectedTargetDirAbs = path.join(rootAbs, CANONICAL_CARGO_TARGET_DIRNAME);
      const requiresCanonicalTargetConfig = repoRequiresCargoTargetConfig(candidateRoot);
      return {
        repoRootAbs: path.resolve(candidateRoot),
        cargoConfigAbs: parsed.cargoConfigAbs,
        exists: parsed.exists,
        requiresCanonicalTargetConfig,
        declaredTargetDir: parsed.declaredTargetDir,
        resolvedTargetDirAbs: parsed.resolvedTargetDirAbs,
        expectedTargetDirAbs,
        matchesCanonicalTarget:
          parsed.exists
          && !!parsed.resolvedTargetDirAbs
          && normalizeComparablePath(parsed.resolvedTargetDirAbs) === normalizeComparablePath(expectedTargetDirAbs),
      };
    })
    .filter((entry) => entry.exists || entry.requiresCanonicalTargetConfig)
    .sort((left, right) => left.repoRootAbs.localeCompare(right.repoRootAbs));

  const siblingArtifactRootDrift = scanSiblingArtifactRootDrift(rootAbs);
  const reclaimableExternalDirs = externalArtifactDirs.filter((entry) => entry.reclaimable);
  const blockingExternalDirs = externalArtifactDirs.filter((entry) => entry.blocking);
  const blockingSiblingArtifactRoots = siblingArtifactRootDrift.filter((entry) => entry.blocking);
  const blockingCargoConfigEntries = cargoTargetConfigs.filter((entry) =>
    entry.requiresCanonicalTargetConfig
      ? !entry.matchesCanonicalTarget
      : entry.exists && !entry.matchesCanonicalTarget
  );
  const blockingIssues = [
    ...repoLocalForbiddenDirs.map((entry) =>
      `repo-local forbidden build artifact directory detected: ${normalizePath(path.relative(entry.repoRootAbs, entry.absPath))} under ${normalizePath(entry.repoRootAbs)}`
    ),
    ...blockingCargoConfigEntries.map((entry) => {
      const cargoConfigRel = normalizePath(path.relative(entry.repoRootAbs, entry.cargoConfigAbs));
      if (!entry.exists) {
        return `${cargoConfigRel}: missing required cargo target-dir config for product checkout; must resolve to ${normalizePath(entry.expectedTargetDirAbs)}`;
      }
      return `${cargoConfigRel}: cargo target-dir must resolve to ${normalizePath(entry.expectedTargetDirAbs)}`;
    }),
    ...blockingSiblingArtifactRoots.map((entry) =>
      `${entry.dirName}: ${entry.reason} (${normalizePath(entry.absPath)})`
    ),
    ...blockingExternalDirs.map((entry) =>
      `${entry.dirName}: ${entry.reason}`
    ),
  ];

  return {
    artifactRootAbs: rootAbs,
    canonicalArtifactDirs: CANONICAL_ARTIFACT_DIRS.map((dirName) => ({
      dirName,
      absPath: path.join(rootAbs, dirName),
      exists: fs.existsSync(path.join(rootAbs, dirName)),
    })),
    repoRoots: repoRoots.map((candidate) => path.resolve(candidate)),
    repoLocalForbiddenDirs,
    cargoTargetConfigs,
    siblingArtifactRootDrift,
    blockingSiblingArtifactRoots,
    externalArtifactDirs,
    reclaimableExternalDirs,
    blockingExternalDirs,
    blockingIssues,
  };
}

export function cleanupArtifactResidue(evaluation, { dryRun = false } = {}) {
  const summary = {
    removedRepoLocalDirs: [],
    removedExternalDirs: [],
    errors: [],
  };
  const rootAbs = path.resolve(evaluation?.artifactRootAbs || "");

  for (const entry of evaluation?.repoLocalForbiddenDirs || []) {
    try {
      if (!dryRun) {
        removeDirSafe(entry.absPath, entry.repoRootAbs);
      }
      summary.removedRepoLocalDirs.push(entry.absPath);
    } catch (error) {
      summary.errors.push(`failed to remove repo-local artifact dir ${normalizePath(entry.absPath)}: ${error.message || error}`);
    }
  }

  for (const entry of evaluation?.reclaimableExternalDirs || []) {
    try {
      if (!dryRun) {
        removeDirSafe(entry.absPath, rootAbs);
      }
      summary.removedExternalDirs.push(entry.absPath);
    } catch (error) {
      summary.errors.push(`failed to remove stale external artifact dir ${normalizePath(entry.absPath)}: ${error.message || error}`);
    }
  }

  return summary;
}

export function artifactRetentionManifestDirAbs(artifactRootAbs) {
  return path.resolve(String(artifactRootAbs || ""), ...ARTIFACT_RETENTION_MANIFEST_DIR_SEGMENTS);
}

export function buildArtifactRetentionManifest({
  repoRoot = REPO_ROOT,
  wpId = "",
  lifecycleScope = "MANUAL_CLEANUP",
  closeoutMode = "",
  actorRole = "",
  actorSession = "",
  dryRun = false,
  generatedAtUtc = new Date().toISOString(),
  artifactEvaluationBeforeCleanup = null,
  artifactCleanupSummary = null,
  artifactEvaluationAfterCleanup = null,
} = {}) {
  const before = artifactEvaluationBeforeCleanup || { artifactRootAbs: resolveArtifactRoot(repoRoot) };
  const after = artifactEvaluationAfterCleanup || before;
  const summary = artifactCleanupSummary || {
    removedRepoLocalDirs: [],
    removedExternalDirs: [],
    errors: [],
  };
  const artifactRootAbs = path.resolve(after.artifactRootAbs || before.artifactRootAbs || resolveArtifactRoot(repoRoot));

  return {
    schema_version: ARTIFACT_RETENTION_MANIFEST_SCHEMA,
    policy_version: ARTIFACT_RETENTION_POLICY_VERSION,
    generated_at_utc: generatedAtUtc,
    repo_root_abs: normalizePath(path.resolve(repoRoot)),
    artifact_root_abs: normalizePath(artifactRootAbs),
    wp_id: String(wpId || "").trim() || null,
    lifecycle_scope: String(lifecycleScope || "").trim() || "MANUAL_CLEANUP",
    closeout_mode: String(closeoutMode || "").trim() || null,
    actor_role: String(actorRole || "").trim() || null,
    actor_session: String(actorSession || "").trim() || null,
    dry_run: Boolean(dryRun),
    policy: {
      canonical_dirs_retained: [...CANONICAL_ARTIFACT_DIRS],
      manifest_dir: normalizePath(path.join(...ARTIFACT_RETENTION_MANIFEST_DIR_SEGMENTS)),
      auto_delete_classes: [
        "repo-local target directories",
        "stale noncanonical external artifact directories classified as NONCANONICAL_EPHEMERAL_STALE",
      ],
      retained_noncanonical_classes: [
        "NONCANONICAL_EPHEMERAL_RECENT",
        "NONCANONICAL_UNKNOWN",
        "NONCANONICAL_SIBLING_ARTIFACT_ROOT",
      ],
      evidence_preservation_rule: "cleanup removes reclaimable residue only; canonical artifact roots and retention manifests remain durable audit surfaces",
    },
    removed_repo_local_dirs: (summary.removedRepoLocalDirs || []).map((entry) => normalizePath(entry)),
    removed_external_dirs: (summary.removedExternalDirs || []).map((entry) => normalizePath(entry)),
    cleanup_errors: (summary.errors || []).map((entry) => String(entry)),
    retained_canonical_dirs: (after.canonicalArtifactDirs || []).map((entry) => ({
      dir_name: entry.dirName,
      abs_path: normalizePath(entry.absPath),
      exists: Boolean(entry.exists),
    })),
    retained_noncanonical_external_dirs: (after.externalArtifactDirs || [])
      .filter((entry) => !CANONICAL_ARTIFACT_DIRS.includes(entry.dirName))
      .map((entry) => ({
        dir_name: entry.dirName,
        abs_path: normalizePath(entry.absPath),
        classification: entry.kind,
        blocking: Boolean(entry.blocking),
        reclaimable: Boolean(entry.reclaimable),
        age_ms: Number.isFinite(entry.ageMs) ? Math.round(entry.ageMs) : null,
        reason: entry.reason,
      })),
    retained_sibling_artifact_roots: (after.siblingArtifactRootDrift || []).map((entry) => ({
      dir_name: entry.dirName,
      abs_path: normalizePath(entry.absPath),
      canonical_artifact_root_abs: normalizePath(entry.canonicalArtifactRootAbs),
      classification: entry.kind,
      blocking: Boolean(entry.blocking),
      reclaimable: Boolean(entry.reclaimable),
      reason: entry.reason,
    })),
    cargo_target_configs: (after.cargoTargetConfigs || []).map((entry) => ({
      repo_root_abs: normalizePath(entry.repoRootAbs),
      cargo_config_abs: normalizePath(entry.cargoConfigAbs),
      required_for_product_checkout: Boolean(entry.requiresCanonicalTargetConfig),
      declared_target_dir: entry.declaredTargetDir || "",
      resolved_target_dir_abs: normalizePath(entry.resolvedTargetDirAbs || ""),
      expected_target_dir_abs: normalizePath(entry.expectedTargetDirAbs || ""),
      matches_canonical_target: Boolean(entry.matchesCanonicalTarget),
    })),
    blocking_issues_after_cleanup: [...(after.blockingIssues || [])],
  };
}

export function writeArtifactRetentionManifest(manifest, { artifactRootAbs = "" } = {}) {
  const resolvedArtifactRootAbs = path.resolve(String(artifactRootAbs || manifest?.artifact_root_abs || ""));
  if (!resolvedArtifactRootAbs) {
    throw new Error("artifact root is required to write an artifact retention manifest");
  }
  const manifestDirAbs = artifactRetentionManifestDirAbs(resolvedArtifactRootAbs);
  fs.mkdirSync(manifestDirAbs, { recursive: true });

  const generatedAtUtc = String(manifest?.generated_at_utc || new Date().toISOString());
  const timestampSegment = sanitizeFileSegment(generatedAtUtc.replace(/:/g, "-"));
  const scopeSegment = sanitizeFileSegment(manifest?.wp_id || manifest?.lifecycle_scope || "manual");
  const modeSegment = sanitizeFileSegment(manifest?.closeout_mode || (manifest?.dry_run ? "dry-run" : "cleanup"));
  const fileName = `${timestampSegment}-${scopeSegment}-${modeSegment}.json`;
  const manifestAbsPath = path.join(manifestDirAbs, fileName);
  fs.writeFileSync(manifestAbsPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

  return {
    manifestAbsPath,
    manifestRelPath: normalizePath(path.relative(resolvedArtifactRootAbs, manifestAbsPath)),
  };
}
