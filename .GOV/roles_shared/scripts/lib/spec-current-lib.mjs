import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

import { GOV_ROOT_REPO_REL } from "./runtime-paths.mjs";

export const SPEC_CURRENT_SCHEMA = "handshake.spec_current@1";
export const SPEC_CURRENT_REPO_REL = `${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md`;

function normalizeRepoPath(value) {
  return String(value || "").trim().replace(/\\/g, "/");
}

function repoAbs(repoRoot, repoRel) {
  return path.resolve(repoRoot, normalizeRepoPath(repoRel));
}

function sha1(buffer) {
  return crypto.createHash("sha1").update(buffer).digest("hex");
}

function sha256(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function readRequiredBuffer(filePathAbs, label) {
  if (!fs.existsSync(filePathAbs)) {
    throw new Error(`Missing ${label}: ${filePathAbs.replace(/\\/g, "/")}`);
  }
  return fs.readFileSync(filePathAbs);
}

function readRequiredText(filePathAbs, label) {
  return readRequiredBuffer(filePathAbs, label).toString("utf8");
}

function parseVersionFromName(fileName) {
  const match = String(fileName || "").match(/_v(\d+(?:\.\d+)*)\.md$/);
  if (!match) return null;
  return {
    tag: `v${match[1]}`,
    parts: match[1].split(".").map((part) => Number(part)),
  };
}

function compareVersionParts(a, b) {
  const maxLen = Math.max(a.length, b.length);
  for (let i = 0; i < maxLen; i += 1) {
    const left = a[i] ?? 0;
    const right = b[i] ?? 0;
    if (left !== right) return left - right;
  }
  return 0;
}

export function findLatestMasterSpecAtRepo(repoRoot) {
  const specDir = repoAbs(repoRoot, `${GOV_ROOT_REPO_REL}/spec`);
  const entries = fs
    .readdirSync(specDir)
    .filter((name) => /^Handshake_Master_Spec_v\d+(?:\.\d+)*\.md$/.test(name))
    .map((name) => ({ name, version: parseVersionFromName(name) }))
    .filter((entry) => entry.version)
    .sort((a, b) => compareVersionParts(a.version.parts, b.version.parts));

  if (entries.length === 0) {
    throw new Error(`No Handshake_Master_Spec_v*.md files found in ${GOV_ROOT_REPO_REL}/spec`);
  }

  const latest = entries[entries.length - 1];
  return {
    name: latest.name,
    versionTag: latest.version.tag,
    path: `${GOV_ROOT_REPO_REL}/spec/${latest.name}`,
    pathAbs: path.join(specDir, latest.name),
  };
}

function parseJsonSpecCurrent(specCurrentText, specCurrentPath) {
  let parsed;
  try {
    parsed = JSON.parse(specCurrentText);
  } catch (error) {
    return { ok: false, error };
  }

  if (parsed?.schema !== SPEC_CURRENT_SCHEMA) {
    throw new Error(`${specCurrentPath}: expected schema ${SPEC_CURRENT_SCHEMA}`);
  }
  if (!parsed.current_spec || typeof parsed.current_spec !== "object") {
    throw new Error(`${specCurrentPath}: missing current_spec object`);
  }
  if (!parsed.governance_reference || typeof parsed.governance_reference !== "object") {
    throw new Error(`${specCurrentPath}: missing governance_reference object`);
  }

  return { ok: true, value: parsed };
}

function orderedManifestModules(manifest) {
  const modules = Array.isArray(manifest.modules) ? manifest.modules : [];
  if (modules.length === 0) {
    throw new Error("indexed spec manifest has no modules");
  }

  const byPath = new Map(modules.map((module) => [normalizeRepoPath(module.path), module]));
  const moduleOrder = Array.isArray(manifest.reconstruction?.module_order)
    ? manifest.reconstruction.module_order.map(normalizeRepoPath)
    : modules.map((module) => normalizeRepoPath(module.path));

  return moduleOrder.map((modulePath) => {
    const module = byPath.get(modulePath);
    if (!module) {
      throw new Error(`indexed spec manifest module_order references missing module: ${modulePath}`);
    }
    return { ...module, path: modulePath };
  });
}

function resolveIndexedManifestAtRepo({ repoRoot, specCurrentPathAbs, specCurrentContract }) {
  const currentSpec = specCurrentContract.current_spec;
  const entrypointPath = normalizeRepoPath(currentSpec.entrypoint_path);
  if (!entrypointPath) {
    throw new Error(`${SPEC_CURRENT_REPO_REL}: current_spec.entrypoint_path is required`);
  }

  const entrypointPathAbs = repoAbs(repoRoot, entrypointPath);
  const manifestText = readRequiredText(entrypointPathAbs, "indexed spec manifest");
  const manifest = JSON.parse(manifestText);
  const manifestDirAbs = path.dirname(entrypointPathAbs);

  const orderedModules = orderedManifestModules(manifest);
  const moduleBuffers = [];
  const modulePaths = [];
  for (const module of orderedModules) {
    const modulePathAbs = path.resolve(manifestDirAbs, module.path);
    const moduleBuffer = readRequiredBuffer(modulePathAbs, `indexed spec module ${module.path}`);
    const actualModuleSha256 = sha256(moduleBuffer);
    if (module.sha256 && module.sha256 !== actualModuleSha256) {
      throw new Error(`indexed spec module hash mismatch: ${module.path}`);
    }
    moduleBuffers.push(moduleBuffer);
    modulePaths.push(normalizeRepoPath(path.relative(repoRoot, modulePathAbs)));
  }

  const reconstructed = Buffer.concat(moduleBuffers);
  const reconstructedSha256 = sha256(reconstructed);
  const expectedReconstructedSha256 = manifest.reconstruction?.reconstructed_sha256;
  if (expectedReconstructedSha256 && expectedReconstructedSha256 !== reconstructedSha256) {
    throw new Error("indexed spec reconstruction hash mismatch");
  }

  const sourceBaselinePath = normalizeRepoPath(
    currentSpec.source_baseline_path || manifest.source?.path || "",
  );
  let sourceBaselineFileName = "";
  let sourceBaselinePathAbs = "";
  if (sourceBaselinePath) {
    sourceBaselinePathAbs = repoAbs(repoRoot, sourceBaselinePath);
    sourceBaselineFileName = path.basename(sourceBaselinePath);
    if (fs.existsSync(sourceBaselinePathAbs) && manifest.source?.sha256) {
      const sourceHash = sha256(fs.readFileSync(sourceBaselinePathAbs));
      if (sourceHash !== manifest.source.sha256) {
        throw new Error(`indexed spec source baseline hash mismatch: ${sourceBaselinePath}`);
      }
    }
  }

  const versionTag =
    normalizeRepoPath(currentSpec.version) ||
    parseVersionFromName(sourceBaselineFileName)?.tag ||
    "";
  const resolverIndexPath = normalizeRepoPath(currentSpec.resolver_index_path || "");
  if (!resolverIndexPath) {
    throw new Error(`${SPEC_CURRENT_REPO_REL}: current_spec.resolver_index_path is required`);
  }

  return {
    format: "json",
    schema: specCurrentContract.schema,
    specCurrentPath: SPEC_CURRENT_REPO_REL,
    specCurrentPathAbs,
    entrypointType: "indexed_manifest",
    specTargetLabel: entrypointPath,
    specEntryPointPath: entrypointPath,
    specEntryPointPathAbs: entrypointPathAbs,
    specFilePath: entrypointPath,
    sourceBaselineFileName,
    sourceBaselinePath,
    sourceBaselinePathAbs,
    resolverIndexPath,
    versionTag,
    sha1: sha1(reconstructed),
    sha256: reconstructedSha256,
    manifest,
    modulePaths,
  };
}

export function resolveSpecCurrentAtRepo(repoRoot, options = {}) {
  const govRoot = normalizeRepoPath(options.govRootRepoRel || GOV_ROOT_REPO_REL);
  const specCurrentPath = `${govRoot}/spec/SPEC_CURRENT.md`;
  const specCurrentPathAbs = repoAbs(repoRoot, specCurrentPath);
  const specCurrentText = readRequiredText(specCurrentPathAbs, "SPEC_CURRENT");
  const parsed = parseJsonSpecCurrent(specCurrentText, specCurrentPath);

  if (parsed.ok) {
    const entrypointType = normalizeRepoPath(parsed.value.current_spec.entrypoint_type);
    if (entrypointType !== "indexed_manifest") {
      throw new Error(`${specCurrentPath}: unsupported current_spec.entrypoint_type ${entrypointType || "<missing>"}`);
    }
    return resolveIndexedManifestAtRepo({
      repoRoot,
      specCurrentPathAbs,
      specCurrentContract: parsed.value,
    });
  }

  throw parsed.error;
}

export function readResolvedSpecTextAtRepo(repoRoot, resolved = resolveSpecCurrentAtRepo(repoRoot)) {
  if (resolved.entrypointType === "indexed_manifest") {
    return resolved.modulePaths
      .map((modulePath) => readRequiredText(repoAbs(repoRoot, modulePath), `indexed spec module ${modulePath}`))
      .join("");
  }

  return readRequiredText(repoAbs(repoRoot, resolved.specFilePath), `spec target ${resolved.specFilePath}`);
}

export function resolveGovernanceReferenceFromSpecCurrentAtRepo(repoRoot, options = {}) {
  const specCurrentPath = normalizeRepoPath(options.specCurrentPath || SPEC_CURRENT_REPO_REL);
  const specCurrentPathAbs = repoAbs(repoRoot, specCurrentPath);
  const specCurrentText = readRequiredText(specCurrentPathAbs, "SPEC_CURRENT");
  const parsed = parseJsonSpecCurrent(specCurrentText, specCurrentPath);

  if (parsed.ok) {
    const codexFilename = normalizeRepoPath(parsed.value.governance_reference.path);
    if (!codexFilename) {
      throw new Error(`${specCurrentPath}: governance_reference.path is required`);
    }
    return {
      codexFilename,
      codexPathAbs: repoAbs(repoRoot, codexFilename),
      specCurrentPathAbs,
    };
  }

  const lines = specCurrentText.split(/\r?\n/);
  const markerIdx = lines.findIndex((line) => /the current authoritative governance reference is\s*:/i.test(line));
  if (markerIdx === -1) throw parsed.error;
  for (let i = markerIdx + 1; i < Math.min(lines.length, markerIdx + 30); i += 1) {
    const match = (lines[i] || "").trim().match(/\*\*(.+?)\*\*/);
    if (match?.[1]) {
      const codexFilename = normalizeRepoPath(match[1]);
      return {
        codexFilename,
        codexPathAbs: repoAbs(repoRoot, codexFilename),
        specCurrentPathAbs,
      };
    }
  }
  throw new Error(`Could not parse governance reference from ${specCurrentPath}`);
}
