#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  activeRules,
  evaluateApplicability,
  loadRegistry,
} from "./hbr-registry-loader.mjs";

const DEFAULT_SCHEMA_VERSION = 1;
const DEFAULT_ADDED_AT_UTC = () => new Date().toISOString();
const PATH_TAG_INFERENCE = new Map([
  ["model_invocation", ["**/model_runtime/**"]],
  ["crdt", ["**/kernel/crdt/**"]],
  ["process_lifecycle", ["**/process_ledger/**"]],
  ["automation_surface", ["app/src-tauri/**", "tests/visual/**"]],
]);
const HYDRATOR_TAG_EXPANSIONS = new Map([
  ["automation_surface", ["automated_run", "headless"]],
]);

function normalizeString(value) {
  return String(value ?? "").trim();
}

function normalizeArray(value) {
  return Array.isArray(value) ? value.filter((entry) => entry !== null && entry !== undefined) : [];
}

function uniqueStrings(values) {
  const seen = new Set();
  const out = [];
  for (const value of normalizeArray(values)) {
    const normalized = normalizeString(value);
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    out.push(normalized);
  }
  return out;
}

function deriveTags(tagsDeclared) {
  const tags = uniqueStrings(tagsDeclared);
  const lowerTags = new Set(tags.map((tag) => tag.toLowerCase()));
  for (const tag of lowerTags) {
    for (const derived of HYDRATOR_TAG_EXPANSIONS.get(tag) || []) {
      if (!lowerTags.has(derived)) {
        tags.push(derived);
        lowerTags.add(derived);
      }
    }
  }
  return tags;
}

function normalizePathForGlob(value) {
  return normalizeString(value).replace(/\\/g, "/").replace(/^\.\/+/, "");
}

function pathMatchesGlob(candidatePath, pattern) {
  const normalizedPath = normalizePathForGlob(candidatePath);
  const normalizedPattern = normalizePathForGlob(pattern);
  if (typeof path.matchesGlob === "function") {
    try {
      return path.matchesGlob(normalizedPath, normalizedPattern);
    } catch {
      // Use the deterministic subset matcher below.
    }
  }
  if (normalizedPattern.startsWith("**/") && normalizedPattern.endsWith("/**")) {
    const segment = normalizedPattern.slice(3, -3);
    return normalizedPath === segment
      || normalizedPath.startsWith(`${segment}/`)
      || normalizedPath.includes(`/${segment}/`)
      || normalizedPath.endsWith(`/${segment}`)
      || normalizedPath.includes(`/${segment}/**`);
  }
  if (normalizedPattern.endsWith("/**")) {
    const prefix = normalizedPattern.slice(0, -3);
    return normalizedPath === prefix
      || normalizedPath.startsWith(`${prefix}/`)
      || normalizedPath.startsWith(`${prefix}/**`);
  }
  return normalizedPath === normalizedPattern;
}

function inferTagsFromTouchedPaths(touchedPaths) {
  const inferred = [];
  for (const [tag, patterns] of PATH_TAG_INFERENCE) {
    if (patterns.some((pattern) => touchedPaths.some((candidate) => pathMatchesGlob(candidate, pattern)))) {
      inferred.push(tag);
    }
  }
  return inferred;
}

function normalizeOverrides(overrides) {
  return normalizeArray(overrides).map((entry, index) => {
    const hbrId = normalizeString(entry?.hbr_id ?? entry?.hbrId);
    const reason = normalizeString(entry?.reason);
    if (!hbrId) {
      throw new Error(`hbr.not_applicable_overrides[${index}] requires hbr_id`);
    }
    if (!reason) {
      throw new Error(`hbr.not_applicable_overrides[${index}] for ${hbrId} requires a non-empty reason`);
    }
    return { hbr_id: hbrId, reason };
  });
}

function buildPacketContext(packet) {
  const hbr = packet?.hbr && typeof packet.hbr === "object" ? packet.hbr : {};
  const scope = packet?.scope && typeof packet.scope === "object" ? packet.scope : {};
  const touchedPaths = uniqueStrings(scope.allowed_paths ?? scope.allowedPaths);
  const declaredTags = [
    ...uniqueStrings(hbr.tags_declared ?? hbr.tagsDeclared),
    ...inferTagsFromTouchedPaths(touchedPaths),
  ];
  if (packet?.requires_foreground === true) {
    declaredTags.push("foreground_required", "exception");
  }
  return {
    wp_id: normalizeString(packet?.wp_id ?? packet?.wpId),
    tags_declared: deriveTags(declaredTags),
    touched_paths: touchedPaths,
    not_applicable_overrides: normalizeOverrides(hbr.not_applicable_overrides ?? hbr.notApplicableOverrides),
  };
}

function ruleOrderIndex(rules) {
  return new Map(rules.map((rule, index) => [normalizeString(rule?.id), index]));
}

function rowSort(order) {
  return (left, right) => {
    const leftId = normalizeString(left?.hbr_id);
    const rightId = normalizeString(right?.hbr_id);
    const leftOrder = order.has(leftId) ? order.get(leftId) : Number.MAX_SAFE_INTEGER;
    const rightOrder = order.has(rightId) ? order.get(rightId) : Number.MAX_SAFE_INTEGER;
    if (leftOrder !== rightOrder) return leftOrder - rightOrder;
    return leftId.localeCompare(rightId);
  };
}

function byHbrId(rows) {
  const map = new Map();
  for (const row of normalizeArray(rows)) {
    const hbrId = normalizeString(row?.hbr_id ?? row?.hbrId);
    if (!hbrId || map.has(hbrId)) continue;
    map.set(hbrId, { ...row, hbr_id: hbrId });
  }
  return map;
}

function applicableRow(rule, existingRows, addedAtUtc) {
  const hbrId = normalizeString(rule?.id);
  const existing = existingRows.get(hbrId);
  if (existing) {
    const status = existing.status || "PENDING";
    // MT-002 finding #1: fail closed on an inconsistent PROVED row so the
    // hydrator can never launder a fake-proved acceptance into the matrix.
    // The handoff gate (hbr/handoff_gate.rs) only accepts a PROVED row when
    // validator_verdict === "PROVED" and evidence_pointer is non-empty; a
    // PROVED row that violates either is corruption (or a manual downgrade in
    // progress) and MUST error rather than be silently preserved.
    if (status === "PROVED") {
      const verdict = normalizeString(existing.validator_verdict);
      const evidence = normalizeString(existing.evidence_pointer);
      if (verdict !== "PROVED" || !evidence) {
        throw new Error(
          `hbr-matrix-hydrate: refusing to hydrate ${hbrId}: status="PROVED" requires `
            + `validator_verdict="PROVED" and a non-empty evidence_pointer `
            + `(got validator_verdict=${JSON.stringify(existing.validator_verdict ?? null)}, `
            + `evidence_pointer=${JSON.stringify(existing.evidence_pointer ?? null)})`,
        );
      }
    }
    return {
      hbr_id: hbrId,
      status,
      evidence_pointer: existing.evidence_pointer ?? null,
      validator_verdict: existing.validator_verdict ?? null,
      added_at_utc: existing.added_at_utc || addedAtUtc,
    };
  }
  return {
    hbr_id: hbrId,
    status: "PENDING",
    evidence_pointer: null,
    validator_verdict: null,
    added_at_utc: addedAtUtc,
  };
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

export function hydratePacketAcceptanceMatrix(packet, options = {}) {
  const registry = options.registry || loadRegistry(options.registryPath);
  const rules = activeRules(registry);
  const order = ruleOrderIndex(rules);
  const addedAtUtc = normalizeString(options.addedAtUtc) || DEFAULT_ADDED_AT_UTC();
  const packetCopy = cloneJson(packet);
  const context = buildPacketContext(packetCopy);
  const overrideIds = new Set(context.not_applicable_overrides.map((entry) => entry.hbr_id));

  const existingMatrix = packetCopy.acceptance_matrix && typeof packetCopy.acceptance_matrix === "object"
    ? packetCopy.acceptance_matrix
    : {};
  const existingApplicableRows = byHbrId(existingMatrix.hbr);
  const nextApplicableRows = [];
  const nextNotApplicableRows = [];

  for (const rule of rules) {
    const hbrId = normalizeString(rule?.id);
    if (!hbrId) continue;
    const result = evaluateApplicability(rule, context);
    if (result.applicability === "Applicable") {
      nextApplicableRows.push(applicableRow(rule, existingApplicableRows, addedAtUtc));
    } else {
      nextNotApplicableRows.push({
        hbr_id: hbrId,
        reason: normalizeString(result.reason),
        source: overrideIds.has(hbrId) ? "operator_override" : "applicability_evaluator",
      });
    }
  }

  for (const [hbrId, row] of existingApplicableRows) {
    if (!order.has(hbrId)) {
      nextApplicableRows.push(row);
    }
  }

  packetCopy.acceptance_matrix = {
    ...existingMatrix,
    schema_version: DEFAULT_SCHEMA_VERSION,
    hbr: nextApplicableRows.sort(rowSort(order)),
    hbr_not_applicable: nextNotApplicableRows.sort(rowSort(order)),
  };

  return packetCopy;
}

export function hydratePacketFile(packetPath, options = {}) {
  const resolvedPath = path.resolve(String(packetPath || ""));
  if (!resolvedPath) {
    throw new Error("--packet <path> is required");
  }
  const packet = JSON.parse(fs.readFileSync(resolvedPath, "utf8"));
  const hydrated = hydratePacketAcceptanceMatrix(packet, options);
  const output = `${JSON.stringify(hydrated, null, 2)}\n`;
  if (!options.dryRun) {
    // MT-002 finding #2/#3: atomic write so a crash mid-write cannot leave a
    // half-written packet (which would then fail gov-check with malformed
    // JSON), and concurrent idempotent hydrations resolve last-writer-wins
    // without corruption. Write a unique temp file in the same directory,
    // then rename (atomic on the same filesystem).
    const tmpPath = `${resolvedPath}.tmp-${process.pid}-${Date.now()}`;
    fs.writeFileSync(tmpPath, output, "utf8");
    try {
      fs.renameSync(tmpPath, resolvedPath);
    } catch (error) {
      try {
        fs.rmSync(tmpPath, { force: true });
      } catch {
        // best-effort cleanup; surface the original rename error
      }
      throw error;
    }
  }
  return { packet: hydrated, output };
}

function parseArgs(argv) {
  const args = {
    packet: null,
    dryRun: false,
    addedAtUtc: null,
    registryPath: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--packet") {
      args.packet = argv[index + 1] || null;
      index += 1;
    } else if (arg === "--dry-run") {
      args.dryRun = true;
    } else if (arg === "--added-at-utc") {
      args.addedAtUtc = argv[index + 1] || null;
      index += 1;
    } else if (arg === "--registry") {
      args.registryPath = argv[index + 1] || null;
      index += 1;
    } else if (arg === "--help" || arg === "-h") {
      args.help = true;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return args;
}

function usage() {
  console.error("Usage: node hbr-matrix-hydrate.mjs --packet <path> [--dry-run] [--registry <path>] [--added-at-utc <iso>]");
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  const invoked = fs.realpathSync.native(path.resolve(process.argv[1]));
  const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
  return invoked === current;
}

if (isInvokedAsMain()) {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help) {
      usage();
      process.exit(0);
    }
    if (!args.packet) {
      usage();
      process.exit(1);
    }
    const result = hydratePacketFile(args.packet, {
      dryRun: args.dryRun,
      addedAtUtc: args.addedAtUtc,
      registryPath: args.registryPath,
    });
    if (args.dryRun) {
      process.stdout.write(result.output);
    } else {
      console.log(`[HBR_MATRIX_HYDRATE] updated ${path.resolve(args.packet)}`);
    }
  } catch (error) {
    console.error(`[HBR_MATRIX_HYDRATE] ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}
