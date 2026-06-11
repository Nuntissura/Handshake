#!/usr/bin/env node

import assert from "node:assert/strict";
import fs from "node:fs";
import nodePath from "node:path";
import { fileURLToPath } from "node:url";

const EXPECTED_SCHEMA = "handshake.build_rules@1";
const EXPECTED_NAME = "HANDSHAKE_BUILD_RULES";
const SEMVER_RE = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;
const DEFAULT_REGISTRY_PATH = fileURLToPath(
  new URL("../records/HANDSHAKE_BUILD_RULES.json", import.meta.url),
);

const TAG_GLOBS = new Map([
  ["model_invocation", ["**/model_runtime/**"]],
  ["crdt", ["**/kernel/crdt/**"]],
  ["process_lifecycle", ["**/process_ledger/**"]],
  ["automation_surface", ["app/src-tauri/**", "tests/visual/**"]],
]);

function normalizeArray(value) {
  return Array.isArray(value) ? value.filter((entry) => entry !== null && entry !== undefined) : [];
}

function normalizePathForGlob(value) {
  return String(value || "").replace(/\\/g, "/").replace(/^\.\/+/, "");
}

function normalizeTag(value) {
  return String(value || "").trim().toLowerCase();
}

function validateRegistryShape(registry, registryPath) {
  if (!registry || typeof registry !== "object" || Array.isArray(registry)) {
    throw new Error(`HBR registry must be a JSON object: ${registryPath}`);
  }
  if (registry.schema !== EXPECTED_SCHEMA) {
    throw new Error(`Invalid HBR registry schema: expected ${EXPECTED_SCHEMA}, got ${registry.schema}`);
  }
  if (registry.name !== EXPECTED_NAME) {
    throw new Error(`Invalid HBR registry name: expected ${EXPECTED_NAME}, got ${registry.name}`);
  }
  if (!SEMVER_RE.test(String(registry.version || ""))) {
    throw new Error(`Invalid HBR registry version: ${registry.version}`);
  }
  if (!Array.isArray(registry.rules)) {
    throw new Error("HBR registry rules must be an array");
  }
  return registry;
}

export function loadRegistry(registryPath = DEFAULT_REGISTRY_PATH) {
  const resolvedPath = nodePath.resolve(String(registryPath || DEFAULT_REGISTRY_PATH));
  const registry = JSON.parse(fs.readFileSync(resolvedPath, "utf8"));
  return validateRegistryShape(registry, resolvedPath);
}

export function activeRules(registry) {
  return normalizeArray(registry?.rules).filter((rule) => String(rule?.status || "").toUpperCase() === "ACTIVE");
}

export function evaluateApplicability(rule, ctx = {}) {
  const ruleId = String(rule?.id || "").trim();
  const overrides = normalizeArray(ctx.not_applicable_overrides ?? ctx.notApplicableOverrides);
  const matchingOverride = overrides.find((entry) =>
    String(entry?.hbr_id ?? entry?.hbrId ?? "").trim() === ruleId
  );
  if (matchingOverride) {
    const reason = String(matchingOverride.reason || "").trim()
      || `not_applicable override matched for ${ruleId}`;
    return { applicability: "NotApplicable", reason };
  }

  const ruleTags = normalizeArray(rule?.applicability?.tags).map(normalizeTag).filter(Boolean);
  const declaredTags = new Set(
    normalizeArray(ctx.tags_declared ?? ctx.tagsDeclared).map(normalizeTag).filter(Boolean),
  );
  if (ruleTags.some((tag) => declaredTags.has(tag))) {
    return { applicability: "Applicable", reason: null };
  }

  const touchedPaths = normalizeArray(ctx.touched_paths ?? ctx.touchedPaths).map(normalizePathForGlob);
  if (ruleTags.some((tag) => tagMatchesTouchedPath(tag, touchedPaths))) {
    return { applicability: "Applicable", reason: null };
  }

  return {
    applicability: "NotApplicable",
    reason: `No declared tags or touched paths matched ${ruleId} for ${String(ctx.wp_id ?? ctx.wpId ?? "").trim() || "<unknown WP>"}.`,
  };
}

function tagMatchesTouchedPath(tag, touchedPaths) {
  const patterns = TAG_GLOBS.get(tag) || [];
  return patterns.some((pattern) => touchedPaths.some((candidate) => pathMatchesGlob(candidate, pattern)));
}

function pathMatchesGlob(candidatePath, pattern) {
  const normalizedPath = normalizePathForGlob(candidatePath);
  if (typeof nodePath.matchesGlob === "function") {
    try {
      return nodePath.matchesGlob(normalizedPath, pattern);
    } catch {
      // Fall through to the deterministic subset matcher below.
    }
  }
  return fallbackPathMatchesGlob(normalizedPath, pattern);
}

function fallbackPathMatchesGlob(candidatePath, pattern) {
  const normalizedPattern = normalizePathForGlob(pattern);
  if (normalizedPattern.startsWith("**/") && normalizedPattern.endsWith("/**")) {
    const segment = normalizedPattern.slice(3, -3);
    return candidatePath === segment
      || candidatePath.startsWith(`${segment}/`)
      || candidatePath.includes(`/${segment}/`)
      || candidatePath.endsWith(`/${segment}`);
  }
  if (normalizedPattern.endsWith("/**")) {
    const prefix = normalizedPattern.slice(0, -3);
    return candidatePath === prefix || candidatePath.startsWith(`${prefix}/`);
  }
  return candidatePath === normalizedPattern;
}

function pillarDistribution(rules) {
  return rules.reduce((distribution, rule) => {
    const pillar = String(rule?.pillar || "").trim();
    distribution[pillar] = (distribution[pillar] || 0) + 1;
    return distribution;
  }, {});
}

function runSelfTest() {
  const registry = loadRegistry();
  const rules = activeRules(registry);
  // Pinned expected shape for registry v1.3.0 (29 active rules). The STOP
  // pillar (5 rules) was added in v1.3.0 for the scope/session-discipline
  // gate [CX-971]; this self-test must track registry version bumps.
  assert.equal(rules.length, 29);
  assert.deepEqual(pillarDistribution(rules), {
    INT: 8,
    SWARM: 4,
    VIS: 5,
    QUIET: 4,
    MAN: 3,
    STOP: 5,
  });

  const observableRule = rules.find((rule) => rule.id === "HBR-INT-001");
  assert(observableRule, "HBR-INT-001 must exist");
  assert.deepEqual(evaluateApplicability(observableRule, {
    wp_id: "WP-HBR-SELF-TEST",
    tags_declared: ["observable_behavior"],
    touched_paths: [],
    not_applicable_overrides: [],
  }), { applicability: "Applicable", reason: null });

  const crdtRule = rules.find((rule) => rule.id === "HBR-INT-004");
  assert(crdtRule, "HBR-INT-004 must exist");
  assert.deepEqual(evaluateApplicability(crdtRule, {
    wp_id: "WP-HBR-SELF-TEST",
    tags_declared: [],
    touched_paths: ["src/backend/handshake_core/src/kernel/crdt/identity.rs"],
    not_applicable_overrides: [],
  }), { applicability: "Applicable", reason: null });

  const overrideResult = evaluateApplicability(observableRule, {
    wp_id: "WP-HBR-SELF-TEST",
    tags_declared: ["observable_behavior"],
    touched_paths: [],
    not_applicable_overrides: [{ hbr_id: "HBR-INT-001", reason: "covered by existing proof" }],
  });
  assert.equal(overrideResult.applicability, "NotApplicable");
  assert.notEqual(overrideResult.reason.trim(), "");

  console.log("[HBR_REGISTRY_LOADER] self-test ok");
  console.log(`- registry: ${DEFAULT_REGISTRY_PATH}`);
  console.log(`- active_rules: ${rules.length}`);
}

function validateCli(registryPath = DEFAULT_REGISTRY_PATH) {
  const registry = loadRegistry(registryPath);
  const rules = activeRules(registry);
  console.log(JSON.stringify({
    schema: registry.schema,
    name: registry.name,
    version: registry.version,
    active_rules: rules.length,
    pillar_distribution: pillarDistribution(rules),
    validation: "ok",
  }, null, 2));
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  const invoked = fs.realpathSync.native(nodePath.resolve(process.argv[1]));
  const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
  return invoked === current;
}

if (isInvokedAsMain()) {
  if (process.argv.includes("--self-test")) {
    runSelfTest();
  } else if (process.argv.includes("--validate")) {
    const registryArgIndex = process.argv.indexOf("--registry");
    const registryPath = registryArgIndex >= 0
      ? process.argv[registryArgIndex + 1] || DEFAULT_REGISTRY_PATH
      : DEFAULT_REGISTRY_PATH;
    validateCli(registryPath);
  } else {
    const registryPath = process.argv[2] || DEFAULT_REGISTRY_PATH;
    const registry = loadRegistry(registryPath);
    console.log(JSON.stringify({
      schema: registry.schema,
      name: registry.name,
      version: registry.version,
      active_rules: activeRules(registry).length,
      pillar_distribution: pillarDistribution(activeRules(registry)),
    }, null, 2));
  }
}
