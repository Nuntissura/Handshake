import fs from "node:fs";
import path from "node:path";
import {
  matchesAnyScopeEntry,
  normalizeRepoPath,
  normalizeScopeEntries,
  parsePacketSingleField,
} from "./scope-surface-lib.mjs";
import { normalizePath, resolveWorkPacketPath } from "./runtime-paths.mjs";

const MICROTASK_FILE_RE = /^MT-\d{3}\.md$/i;

function normalizeScopeRefKey(value) {
  return String(value || "")
    .trim()
    .replace(/^`|`$/g, "")
    .replace(/\s+/g, " ")
    .replace(/\/+/g, "/")
    .toUpperCase();
}

function parseSemicolonList(rawValue, { normalizeAsRepoPath = false } = {}) {
  const entries = String(rawValue || "")
    .split(";")
    .map((value) => value.trim())
    .filter(Boolean);
  if (!normalizeAsRepoPath) return Array.from(new Set(entries));
  return normalizeScopeEntries(entries);
}

function parseMicrotaskDefinition(mtAbsPath, mtRelPath) {
  const text = fs.readFileSync(mtAbsPath, "utf8");
  const mtId = String(parsePacketSingleField(text, "MT_ID") || "").trim();
  const clause = String(parsePacketSingleField(text, "CLAUSE") || "").trim();
  const codeSurfaces = parseSemicolonList(parsePacketSingleField(text, "CODE_SURFACES"), { normalizeAsRepoPath: true });
  const expectedTests = parseSemicolonList(parsePacketSingleField(text, "EXPECTED_TESTS"));
  const dependsOn = String(parsePacketSingleField(text, "DEPENDS_ON") || "").trim() || "NONE";

  if (!mtId) {
    throw new Error(`Malformed microtask file ${normalizePath(mtRelPath)}: missing MT_ID`);
  }

  const clauseTokenMatches = Array.from(clause.matchAll(/\[([^\]]+)\]/g))
    .map((match) => String(match[1] || "").trim())
    .filter(Boolean);
  const aliases = new Set([
    mtId,
    clause,
    `CLAUSE_CLOSURE_MATRIX/${clause}`,
  ]);
  for (const token of clauseTokenMatches) {
    aliases.add(token);
    aliases.add(`[${token}]`);
    aliases.add(`CLAUSE_CLOSURE_MATRIX/${token}`);
    aliases.add(`CLAUSE_CLOSURE_MATRIX/[${token}]`);
  }

  return {
    mtId,
    clause,
    codeSurfaces,
    expectedTests,
    dependsOn,
    packetPath: normalizePath(mtRelPath),
    scopeRefKeys: Array.from(aliases).map((value) => normalizeScopeRefKey(value)).filter(Boolean),
  };
}

export function listDeclaredWpMicrotasks(wpId) {
  const resolved = resolveWorkPacketPath(wpId);
  if (!resolved?.isFolder || !resolved.packetDirAbs || !fs.existsSync(resolved.packetDirAbs)) {
    return [];
  }

  return fs.readdirSync(resolved.packetDirAbs, { withFileTypes: true })
    .filter((entry) => entry.isFile() && MICROTASK_FILE_RE.test(entry.name))
    .sort((left, right) => left.name.localeCompare(right.name))
    .map((entry) => {
      const mtAbsPath = path.join(resolved.packetDirAbs, entry.name);
      const mtRelPath = path.posix.join(resolved.packetDir, entry.name);
      return parseMicrotaskDefinition(mtAbsPath, mtRelPath);
    });
}

export function resolveDeclaredWpMicrotaskByScopeRef(wpId, scopeRef, microtasks = null) {
  const declaredMicrotasks = Array.isArray(microtasks) ? microtasks : listDeclaredWpMicrotasks(wpId);
  const scopeRefKey = normalizeScopeRefKey(scopeRef);
  if (!scopeRefKey) {
    return {
      declaredMicrotasks,
      match: null,
      ambiguousMatches: [],
    };
  }

  const matches = declaredMicrotasks.filter((definition) => definition.scopeRefKeys.includes(scopeRefKey));
  return {
    declaredMicrotasks,
    match: matches.length === 1 ? matches[0] : null,
    ambiguousMatches: matches.length > 1 ? matches : [],
  };
}

export function summarizeMicrotaskFileTargetBudget(fileTargets, microtaskDefinition) {
  const normalizedTargets = Array.isArray(fileTargets)
    ? fileTargets.map((entry) => normalizeRepoPath(entry)).filter(Boolean)
    : [];
  const allowedSurfaces = Array.isArray(microtaskDefinition?.codeSurfaces)
    ? microtaskDefinition.codeSurfaces
    : [];
  const outOfBudgetTargets = normalizedTargets.filter((target) => !matchesAnyScopeEntry(target, allowedSurfaces));

  return {
    normalizedTargets,
    allowedSurfaces,
    outOfBudgetTargets,
    ok: outOfBudgetTargets.length === 0,
  };
}
