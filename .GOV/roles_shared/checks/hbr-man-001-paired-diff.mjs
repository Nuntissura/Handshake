#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const MANUAL_CONTENT_PATH = "src/backend/handshake_core/src/model_manual/content.rs";
const MANUAL_MOD_PATH = "src/backend/handshake_core/src/model_manual/mod.rs";

class CliError extends Error {
  constructor(message) {
    super(message);
    this.name = "CliError";
  }
}

class GitError extends Error {
  constructor(message) {
    super(message);
    this.name = "GitError";
  }
}

function toPosix(value) {
  return String(value || "").replace(/\\/g, "/");
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function parseArgs(args) {
  const options = {
    baseRef: "HEAD~1",
    targetRef: "HEAD",
    repoRoot: String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim() || process.cwd(),
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--base-ref") {
      const value = args[index + 1];
      if (!isNonEmptyString(value)) throw new CliError("--base-ref requires a git ref");
      options.baseRef = value.trim();
      index += 1;
      continue;
    }
    if (arg === "--target-ref") {
      const value = args[index + 1];
      if (!isNonEmptyString(value)) throw new CliError("--target-ref requires a git ref");
      options.targetRef = value.trim();
      index += 1;
      continue;
    }
    if (arg === "--repo-root") {
      const value = args[index + 1];
      if (!isNonEmptyString(value)) throw new CliError("--repo-root requires a path");
      options.repoRoot = path.resolve(value);
      index += 1;
      continue;
    }
    throw new CliError(`unknown argument: ${arg}`);
  }

  return options;
}

function git(repoRoot, args, allowFailure = false) {
  try {
    return execFileSync("git", ["-C", repoRoot, ...args], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    if (allowFailure) return null;
    const stderr = String(error?.stderr || "").trim();
    const stdout = String(error?.stdout || "").trim();
    const detail = stderr || stdout || String(error?.message || error || "unknown git error");
    throw new GitError(`git ${args.join(" ")} failed: ${detail}`);
  }
}

function resolveGitRoot(repoRoot) {
  const resolved = path.resolve(repoRoot);
  const gitRoot = git(resolved, ["rev-parse", "--show-toplevel"]).trim();
  if (!gitRoot) throw new GitError(`unable to resolve git root for ${resolved}`);
  return path.resolve(gitRoot);
}

function diffRange(baseRef, targetRef) {
  return `${baseRef}..${targetRef}`;
}

function changedFiles(repoRoot, baseRef, targetRef) {
  const output = git(repoRoot, ["diff", "--name-only", "-z", "--diff-filter=ACDMRT", diffRange(baseRef, targetRef)]);
  return output.split("\0").map((line) => toPosix(line)).filter(Boolean);
}

function filePatch(repoRoot, baseRef, targetRef, relativeFile) {
  return git(repoRoot, [
    "diff",
    "--no-color",
    "--no-ext-diff",
    "--find-renames",
    "--unified=80",
    diffRange(baseRef, targetRef),
    "--",
    relativeFile,
  ]);
}

function showFile(repoRoot, ref, relativeFile) {
  return git(repoRoot, ["show", `${ref}:${relativeFile}`], true);
}

function changedPatchLines(patch) {
  return patch.split(/\r?\n/)
    .filter((line) => (line.startsWith("+") || line.startsWith("-")) && !line.startsWith("+++") && !line.startsWith("---"))
    .map((line) => ({
      sign: line[0],
      text: line.slice(1),
    }));
}

function stripDiffLine(line) {
  if (!line) return "";
  if ((line.startsWith("+") || line.startsWith("-")) && !line.startsWith("+++") && !line.startsWith("---")) {
    return line.slice(1);
  }
  if (line.startsWith(" ")) return line.slice(1);
  return line;
}

function nearbyFunctionName(lines, startIndex, sign) {
  for (let index = startIndex + 1; index < Math.min(lines.length, startIndex + 16); index += 1) {
    const raw = lines[index];
    if (raw.startsWith("@@") || raw.startsWith("diff --git")) break;
    if (raw.startsWith("+++") || raw.startsWith("---")) continue;
    if (raw[0] !== sign && raw[0] !== " ") continue;
    const text = stripDiffLine(raw);
    const match = text.match(/\b(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/);
    if (match) return match[1];
  }
  return null;
}

function hasNearbyTauriCommandAttribute(lines, startIndex) {
  for (let index = Math.max(0, startIndex - 8); index <= startIndex; index += 1) {
    const raw = lines[index];
    if (raw.startsWith("@@") || raw.startsWith("diff --git")) continue;
    if (stripDiffLine(raw).match(/#\s*\[\s*tauri::command\b/)) return true;
  }
  return false;
}

function addSurface(surfaces, seen, file, surfaceKind, name) {
  const normalizedName = String(name || "<unknown>").trim() || "<unknown>";
  const key = `${file}\0${surfaceKind}\0${normalizedName}`;
  if (seen.has(key)) return;
  seen.add(key);
  surfaces.push({
    file,
    surface_kind: surfaceKind,
    name: normalizedName,
  });
}

function detectTauriCommands(file, patch, surfaces, seen) {
  const lines = patch.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const raw = lines[index];
    if (!raw || raw.startsWith("+++") || raw.startsWith("---")) continue;
    if ((raw.startsWith("+") || raw.startsWith("-")) && stripDiffLine(raw).match(/#\s*\[\s*tauri::command\b/)) {
      addSurface(surfaces, seen, file, "tauri_command", nearbyFunctionName(lines, index, raw[0]));
      continue;
    }
    if ((raw.startsWith("+") || raw.startsWith("-")) && stripDiffLine(raw).match(/\b(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\(/)) {
      if (!hasNearbyTauriCommandAttribute(lines, index)) continue;
      const match = stripDiffLine(raw).match(/\b(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/);
      addSurface(surfaces, seen, file, "tauri_command", match?.[1]);
    }
  }
}

function detectIpcChannels(file, patch, surfaces, seen) {
  for (const line of changedPatchLines(patch)) {
    for (const match of line.text.matchAll(/"((?:kernel)\.[a-z_]+\.[a-z_]+)"/g)) {
      addSurface(surfaces, seen, file, "ipc_channel", match[1]);
    }
  }
}

function patchHasSerializableStructContext(patch, baseContent, targetContent) {
  const searchable = [patch, baseContent || "", targetContent || ""].join("\n");
  return /\bSerialize\b/.test(searchable)
    && /\bDeserialize\b/.test(searchable)
    && /\bstruct\s+[A-Za-z_][A-Za-z0-9_]*/.test(searchable);
}

function detectSchemaFields(file, patch, surfaces, seen, baseContent, targetContent) {
  if (!file.endsWith("/types.rs") || !patchHasSerializableStructContext(patch, baseContent, targetContent)) return;
  for (const line of changedPatchLines(patch)) {
    const match = line.text.match(/^\s*(?:pub\s+)?([a-zA-Z_][A-Za-z0-9_]*)\s*:/);
    if (!match) continue;
    addSurface(surfaces, seen, file, "schema_field", match[1]);
  }
}

function cliFlagNameFromAttribute(text) {
  const explicitLong = text.match(/\blong\s*=\s*"([^"]+)"/);
  if (explicitLong) return explicitLong[1];
  const explicitShort = text.match(/\bshort\s*=\s*'([^']+)'/);
  if (explicitShort) return explicitShort[1];
  return null;
}

function nearbyFieldName(lines, startIndex, sign) {
  for (let index = startIndex + 1; index < Math.min(lines.length, startIndex + 10); index += 1) {
    const raw = lines[index];
    if (raw.startsWith("@@") || raw.startsWith("diff --git")) break;
    if (raw[0] !== sign && raw[0] !== " ") continue;
    const match = stripDiffLine(raw).match(/^\s*(?:pub\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:/);
    if (match) return match[1].replaceAll("_", "-");
  }
  return null;
}

function detectCliFlags(file, patch, surfaces, seen) {
  const lines = patch.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const raw = lines[index];
    if (!raw || raw.startsWith("+++") || raw.startsWith("---")) continue;
    if (!raw.startsWith("+") && !raw.startsWith("-")) continue;
    const text = stripDiffLine(raw);
    if (!/#\s*\[\s*(?:arg|clap)\s*\(/.test(text)) continue;
    const name = cliFlagNameFromAttribute(text) || nearbyFieldName(lines, index, raw[0]);
    addSurface(surfaces, seen, file, "cli_flag", name);
  }
}

function detectConfigKeys(file, patch, surfaces, seen) {
  // MT-011: only emit config_key for fields that actually live inside the BODY of
  // a *Config / *Settings / *Options struct definition (or any struct body in a
  // config/settings/options file). The prior implementation flagged EVERY "name:"
  // changed line in any patch that merely *contained* such a struct anywhere,
  // which flooded ordinary struct fields, enum variants, and struct-literal locals
  // (e.g. 455 false config_key hits on workflows.rs). We brace-track struct bodies
  // across the patch (context + changed lines) and attribute only changed field
  // lines inside a config-relevant struct definition body.
  const configLikeFile = /(^|\/)(config|settings|options)(\/|\.rs$)/i.test(file);
  const lines = patch.split(/\r?\n/);
  let depth = 0; // best-effort running brace depth across the patch hunk(s)
  const structStack = []; // { baseDepth, configRelevant } per open struct-definition body
  for (const raw of lines) {
    if (!raw || raw.startsWith("+++") || raw.startsWith("---") || raw.startsWith("@@")) continue;
    const isChanged = raw.startsWith("+") || raw.startsWith("-");
    const text = stripDiffLine(raw);
    const structMatch = text.match(/\bstruct\s+([A-Za-z_][A-Za-z0-9_]*)\b/);
    const opensThisLine = (text.match(/\{/g) || []).length;
    const closesThisLine = (text.match(/\}/g) || []).length;

    // A struct definition body opens on a `struct Name {` line (with `struct`
    // keyword + an open brace). Struct LITERALS (`SomeConfig { .. }`) and enum
    // bodies (`enum E {`, variant `V { .. }`) do not match and are not pushed.
    if (structMatch && opensThisLine > 0) {
      const name = structMatch[1];
      const configRelevant = configLikeFile || /(?:Config|Settings|Options)$/.test(name);
      structStack.push({ baseDepth: depth, configRelevant });
    }

    // Inside the innermost config-relevant struct body, a changed field line is a config key.
    const top = structStack[structStack.length - 1];
    if (isChanged && top && top.configRelevant && depth > top.baseDepth) {
      const fieldMatch = text.match(/^\s*(?:pub(?:\s*\([^)]*\))?\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:/);
      if (fieldMatch) {
        addSurface(surfaces, seen, file, "config_key", fieldMatch[1]);
      }
    }

    depth += opensThisLine - closesThisLine;
    while (structStack.length && depth <= structStack[structStack.length - 1].baseDepth) {
      structStack.pop();
    }
  }
}

function isRuntimeRustFile(file) {
  const normalized = toPosix(file);
  if (!normalized.endsWith(".rs")) return false;
  if (/(^|\/)(tests?|benches|examples)\//.test(normalized)) return false;
  if (/(^|\/)[^/]*(?:_test|_tests)\.rs$/.test(normalized)) return false;
  return true;
}

function detectWiredSurfaces(repoRoot, baseRef, targetRef, files) {
  const surfaces = [];
  const seen = new Set();
  for (const file of files.filter(isRuntimeRustFile)) {
    const patch = filePatch(repoRoot, baseRef, targetRef, file);
    const baseContent = showFile(repoRoot, baseRef, file);
    const targetContent = showFile(repoRoot, targetRef, file);
    detectTauriCommands(file, patch, surfaces, seen);
    detectIpcChannels(file, patch, surfaces, seen);
    detectSchemaFields(file, patch, surfaces, seen, baseContent, targetContent);
    detectCliFlags(file, patch, surfaces, seen);
    detectConfigKeys(file, patch, surfaces, seen);
  }
  return surfaces;
}

function extractManualVersion(content) {
  if (content === null) return null;
  const match = String(content).match(/\bMANUAL_VERSION\b[^=]*=\s*"([^"]+)"/);
  return match ? match[1] : null;
}

function parseSemver(version) {
  const match = String(version || "").trim().match(/^(\d+)\.(\d+)\.(\d+)(?:[-+][0-9A-Za-z.-]+)?$/);
  if (!match) return null;
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    raw: String(version).trim(),
  };
}

function semverDelta(baseVersion, targetVersion) {
  const base = parseSemver(baseVersion);
  const target = parseSemver(targetVersion);
  if (!base || !target) {
    return {
      level: "none",
      higher: false,
      reason: `MANUAL_VERSION must be valid SemVer in both refs (base=${baseVersion || "missing"}, target=${targetVersion || "missing"})`,
    };
  }

  if (target.major > base.major) return { level: "major", higher: true, base, target };
  if (target.major < base.major) return { level: "none", higher: false, base, target };
  if (target.minor > base.minor) return { level: "minor", higher: true, base, target };
  if (target.minor < base.minor) return { level: "none", higher: false, base, target };
  if (target.patch > base.patch) return { level: "patch", higher: true, base, target };
  return { level: "none", higher: false, base, target };
}

function manualContentPatch(repoRoot, baseRef, targetRef, changed) {
  if (!changed.has(MANUAL_CONTENT_PATH)) return "";
  return filePatch(repoRoot, baseRef, targetRef, MANUAL_CONTENT_PATH);
}

function requiredBumpLevel(contentPatch) {
  const changed = changedPatchLines(contentPatch);
  if (changed.some((line) => line.sign === "-" && /\bCommandReference\b/.test(line.text))) return "major";
  if (changed.some((line) => /\b(?:ManualWorkflow|ManualSafetyConstraint)\b/.test(line.text))) return "minor";
  return "patch";
}

function bumpSatisfies(deltaLevel, requiredLevel) {
  const rank = { none: 0, patch: 1, minor: 2, major: 3 };
  return rank[deltaLevel] >= rank[requiredLevel];
}

function surfaceManualTokens(surface) {
  const rawName = String(surface?.name || "").trim();
  if (!rawName || rawName === "<unknown>") return [];
  const tokens = new Set([rawName]);
  tokens.add(rawName.replaceAll("_", "-"));
  tokens.add(rawName.replaceAll("-", "_"));
  tokens.add(rawName.replaceAll("_", "."));
  tokens.add(rawName.replaceAll(".", "_"));
  tokens.add(rawName.replaceAll(".", "-"));
  if (surface?.surface_kind === "cli_flag") {
    tokens.add(`--${rawName.replaceAll("_", "-")}`);
  }
  return [...tokens].map((token) => token.toLowerCase()).filter(Boolean);
}

function manualPatchMentionsSurface(surface, contentPatch) {
  const tokens = surfaceManualTokens(surface);
  if (tokens.length === 0) return false;
  const changedText = changedPatchLines(contentPatch)
    .map((line) => line.text)
    .join("\n")
    .toLowerCase();
  return tokens.some((token) => changedText.includes(token));
}

function shortCommit(commitRef) {
  return commitRef ? String(commitRef).slice(0, 12) : null;
}

function sameCommitReason(reason, commitRef) {
  if (!commitRef) return reason;
  return `${reason}; each wired surface and ModelManual update must occur in the same commit (${shortCommit(commitRef)})`;
}

function failureFor(surface, reason, commitRef) {
  const record = {
    file: surface.file,
    surface_kind: surface.surface_kind,
    name: surface.name,
    reason: sameCommitReason(reason, commitRef),
  };
  const commit = shortCommit(commitRef);
  if (commit) record.commit = commit;
  return record;
}

function policyFailures({ repoRoot, baseRef, targetRef, commitRef = null, files, surfaces }) {
  if (surfaces.length === 0) return [];

  const changed = new Set(files);
  if (!changed.has(MANUAL_CONTENT_PATH)) {
    return surfaces.map((surface) => failureFor(
      surface,
      `wired surface changed without paired ModelManual ${MANUAL_CONTENT_PATH} diff`,
      commitRef,
    ));
  }

  const contentPatch = manualContentPatch(repoRoot, baseRef, targetRef, changed);
  const unmatched = surfaces.filter((surface) => !manualPatchMentionsSurface(surface, contentPatch));
  if (unmatched.length > 0) {
    return unmatched.map((surface) => failureFor(
      surface,
      `wired surface changed without corresponding ModelManual diff mentioning ${surface.name}`,
      commitRef,
    ));
  }

  const baseManualMod = showFile(repoRoot, baseRef, MANUAL_MOD_PATH);
  const targetManualMod = showFile(repoRoot, targetRef, MANUAL_MOD_PATH);
  const baseVersion = extractManualVersion(baseManualMod);
  const targetVersion = extractManualVersion(targetManualMod);
  const delta = semverDelta(baseVersion, targetVersion);
  const requiredLevel = requiredBumpLevel(contentPatch);
  if (!delta.higher || !bumpSatisfies(delta.level, requiredLevel)) {
    const detail = delta.reason || `base=${baseVersion || "missing"}, target=${targetVersion || "missing"}`;
    return surfaces.map((surface) => failureFor(
      surface,
      `wired surface changed with ModelManual content diff but MANUAL_VERSION did not bump higher at required ${requiredLevel.toUpperCase()} level (${detail})`,
      commitRef,
    ));
  }

  return [];
}

function commitRefs(repoRoot, baseRef, targetRef) {
  const output = git(repoRoot, ["rev-list", "--reverse", diffRange(baseRef, targetRef)]);
  return output.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
}

function firstParentRef(repoRoot, commitRef) {
  return git(repoRoot, ["rev-parse", "--verify", `${commitRef}^`]).trim();
}

function evaluatePolicyPair({ repoRoot, baseRef, targetRef, commitRef = null }) {
  const files = changedFiles(repoRoot, baseRef, targetRef);
  const surfaces = detectWiredSurfaces(repoRoot, baseRef, targetRef, files);
  const failures = policyFailures({
    repoRoot,
    baseRef,
    targetRef,
    commitRef,
    files,
    surfaces,
  });
  return { surfaces, failures };
}

function emitJsonLines(records) {
  for (const record of records) {
    console.error(JSON.stringify(record));
  }
}

function errorRecord(error) {
  return {
    file: null,
    surface_kind: null,
    name: null,
    reason: error?.message || String(error || "unknown error"),
  };
}

export function runCli(args = process.argv.slice(2)) {
  let options;
  try {
    options = parseArgs(args);
    options.repoRoot = resolveGitRoot(options.repoRoot);
    git(options.repoRoot, ["rev-parse", "--verify", `${options.baseRef}^{commit}`]);
    git(options.repoRoot, ["rev-parse", "--verify", `${options.targetRef}^{commit}`]);
  } catch (error) {
    emitJsonLines([errorRecord(error)]);
    return 3;
  }

  try {
    const commits = commitRefs(options.repoRoot, options.baseRef, options.targetRef);
    const pairs = commits.length > 0
      ? commits.map((commitRef) => ({
        baseRef: firstParentRef(options.repoRoot, commitRef),
        targetRef: commitRef,
        commitRef,
      }))
      : [{ baseRef: options.baseRef, targetRef: options.targetRef, commitRef: null }];

    let surfaceCount = 0;
    const failures = [];
    for (const pair of pairs) {
      const result = evaluatePolicyPair({ repoRoot: options.repoRoot, ...pair });
      surfaceCount += result.surfaces.length;
      failures.push(...result.failures);
    }

    if (failures.length > 0) {
      emitJsonLines(failures);
      return 2;
    }

    const noun = surfaceCount === 1 ? "wired surface change" : "wired surface changes";
    console.log(`hbr-man-001-paired-diff ok (${surfaceCount} ${noun})`);
    return 0;
  } catch (error) {
    emitJsonLines([errorRecord(error)]);
    return 3;
  }
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  const invoked = fs.realpathSync.native(path.resolve(process.argv[1]));
  const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
  return invoked === current;
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
