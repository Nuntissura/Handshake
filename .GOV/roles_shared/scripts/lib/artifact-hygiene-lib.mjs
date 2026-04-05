import fs from "node:fs";
import path from "node:path";
import { discoverGitCheckouts } from "../topology/git-topology-lib.mjs";
import { REPO_ROOT, normalizePath } from "./runtime-paths.mjs";

export const HANDSHAKE_ARTIFACT_ROOT_ENV_VAR = "HANDSHAKE_ARTIFACT_ROOT";
export const DEFAULT_ARTIFACT_ROOT_DIRNAME = "Handshake Artifacts";
export const CANONICAL_ARTIFACT_DIRS = Object.freeze([
  "handshake-cargo-target",
  "handshake-product",
  "handshake-test",
  "handshake-tool",
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
      const expectedTargetDirAbs = path.join(rootAbs, "handshake-cargo-target");
      return {
        repoRootAbs: path.resolve(candidateRoot),
        cargoConfigAbs: parsed.cargoConfigAbs,
        exists: parsed.exists,
        declaredTargetDir: parsed.declaredTargetDir,
        resolvedTargetDirAbs: parsed.resolvedTargetDirAbs,
        expectedTargetDirAbs,
        matchesCanonicalTarget:
          parsed.exists
          && !!parsed.resolvedTargetDirAbs
          && normalizeComparablePath(parsed.resolvedTargetDirAbs) === normalizeComparablePath(expectedTargetDirAbs),
      };
    })
    .filter((entry) => entry.exists)
    .sort((left, right) => left.repoRootAbs.localeCompare(right.repoRootAbs));

  const reclaimableExternalDirs = externalArtifactDirs.filter((entry) => entry.reclaimable);
  const blockingExternalDirs = externalArtifactDirs.filter((entry) => entry.blocking);
  const blockingCargoConfigEntries = cargoTargetConfigs.filter((entry) => !entry.matchesCanonicalTarget);
  const blockingIssues = [
    ...repoLocalForbiddenDirs.map((entry) =>
      `repo-local forbidden build artifact directory detected: ${normalizePath(path.relative(entry.repoRootAbs, entry.absPath))} under ${normalizePath(entry.repoRootAbs)}`
    ),
    ...blockingCargoConfigEntries.map((entry) =>
      `${normalizePath(path.relative(entry.repoRootAbs, entry.cargoConfigAbs))}: cargo target-dir must resolve to ${normalizePath(entry.expectedTargetDirAbs)}`
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
