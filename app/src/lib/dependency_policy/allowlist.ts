// WP-KERNEL-009 / MT-017 — RuntimeDependencyAllowlist (typed accessor).
//
// Single typed entry point over the machine-readable allowlist document
// `runtime_dependency_allowlist.json`. The JSON document is the data
// authority; this module validates its shape at import time so every
// consumer (tests, tripwire scanners, the MT-032 validator hook) fails
// loudly if the document drifts instead of silently scanning nothing.
//
// Consumers:
//   - allowlist.test.ts                  (MT-017 consumption gate)
//   - bundled_library_policy.test.ts     (MT-018)
//   - package_lock_audit.test.ts         (MT-019)
//   - docker/sqlite tripwire tests       (MT-024 / MT-025)
//   - app/scripts/lib/dependency_policy_scans.mjs (shared scanners)
//   - app/scripts/check-dependency-policy.mjs     (MT-032 validator hook)
//   - src/backend/handshake_core/src/dependency_policy/mod.rs (Rust parity)

import rawAllowlist from "./runtime_dependency_allowlist.json";

export const RUNTIME_DEPENDENCY_ALLOWLIST_SCHEMA =
  "handshake.runtime_dependency_allowlist@1" as const;

export type ExternalRuntimeInputKind =
  | "model_gguf"
  | "model_safetensors"
  | "tensor_artifact"
  | "cui_portable_artifact";

export const EXTERNAL_RUNTIME_INPUT_KINDS: readonly ExternalRuntimeInputKind[] = [
  "model_gguf",
  "model_safetensors",
  "tensor_artifact",
  "cui_portable_artifact",
];

export interface AllowedExternalRuntimeInput {
  kind: ExternalRuntimeInputKind;
  description: string;
  operator_gated: boolean;
  default_enabled: boolean;
  allowed_extensions: readonly string[];
  owning_surface: string;
}

export interface ForbiddenRuntimeDependencyClass {
  id: string;
  description: string;
  /** Exact npm package names that must never appear in product manifests. */
  npm_package_names: readonly string[];
  /** Lowercase substrings; any cargo crate whose name contains one is forbidden. */
  cargo_crate_name_substrings: readonly string[];
  /** Lowercase substrings scanned for in product source files. */
  source_scan_patterns: readonly string[];
}

export interface BundledLibraryRule {
  family: string;
  ecosystem: "npm" | "cargo";
  /** Exact names, or prefix patterns ending in `*`. */
  package_patterns: readonly string[];
  allowed_licenses: readonly string[];
  reason: string;
}

export interface DockerOptInException {
  path_prefix: string;
  reason: string;
}

export interface ProductManifests {
  npm: readonly string[];
  npm_lockfiles: readonly string[];
  cargo: readonly string[];
  cargo_lockfiles: readonly string[];
}

export interface RuntimeDependencyAllowlist {
  schema: typeof RUNTIME_DEPENDENCY_ALLOWLIST_SCHEMA;
  version: string;
  wp_id: string;
  mt_id: string;
  description: string;
  allowed_external_runtime_inputs: readonly AllowedExternalRuntimeInput[];
  forbidden_runtime_dependency_classes: readonly ForbiddenRuntimeDependencyClass[];
  bundled_libraries: readonly BundledLibraryRule[];
  docker_opt_in_exceptions: readonly DockerOptInException[];
  product_scan_roots: readonly string[];
  product_manifests: ProductManifests;
}

export class AllowlistShapeError extends Error {
  constructor(message: string) {
    super(`runtime_dependency_allowlist.json invalid: ${message}`);
    this.name = "AllowlistShapeError";
  }
}

function assertCondition(condition: boolean, message: string): asserts condition {
  if (!condition) throw new AllowlistShapeError(message);
}

/**
 * Validates the raw JSON document shape. Throws AllowlistShapeError on any
 * structural drift so consumers can never operate on a half-formed policy.
 */
export function validateAllowlistDocument(doc: unknown): RuntimeDependencyAllowlist {
  assertCondition(typeof doc === "object" && doc !== null, "document is not an object");
  const d = doc as Record<string, unknown>;
  assertCondition(
    d.schema === RUNTIME_DEPENDENCY_ALLOWLIST_SCHEMA,
    `schema must be ${RUNTIME_DEPENDENCY_ALLOWLIST_SCHEMA}, got ${String(d.schema)}`,
  );
  assertCondition(typeof d.version === "string" && d.version.length > 0, "version missing");
  assertCondition(typeof d.wp_id === "string" && d.wp_id.length > 0, "wp_id missing");
  assertCondition(typeof d.mt_id === "string" && d.mt_id.length > 0, "mt_id missing");

  const inputs = d.allowed_external_runtime_inputs;
  assertCondition(Array.isArray(inputs) && inputs.length > 0, "allowed_external_runtime_inputs empty");
  const kinds = new Set<string>();
  for (const input of inputs as Array<Record<string, unknown>>) {
    assertCondition(
      typeof input.kind === "string" &&
        (EXTERNAL_RUNTIME_INPUT_KINDS as readonly string[]).includes(input.kind),
      `unknown external runtime input kind: ${String(input.kind)}`,
    );
    assertCondition(!kinds.has(input.kind as string), `duplicate input kind ${String(input.kind)}`);
    kinds.add(input.kind as string);
    assertCondition(input.operator_gated === true, `input ${String(input.kind)} must be operator_gated`);
    assertCondition(
      input.default_enabled === false,
      `input ${String(input.kind)} must default to disabled`,
    );
    assertCondition(Array.isArray(input.allowed_extensions), `input ${String(input.kind)} missing allowed_extensions`);
    assertCondition(
      typeof input.owning_surface === "string" && input.owning_surface.length > 0,
      `input ${String(input.kind)} missing owning_surface`,
    );
  }
  assertCondition(
    kinds.size === EXTERNAL_RUNTIME_INPUT_KINDS.length,
    `expected exactly ${EXTERNAL_RUNTIME_INPUT_KINDS.length} input kinds, got ${kinds.size}`,
  );

  const forbidden = d.forbidden_runtime_dependency_classes;
  assertCondition(Array.isArray(forbidden) && forbidden.length > 0, "forbidden classes empty");
  const requiredForbiddenIds = [
    "outside_app",
    "outside_server_daemon",
    "docker_default",
    "sqlite",
    "cdn_runtime_asset",
  ];
  const forbiddenIds = new Set(
    (forbidden as Array<Record<string, unknown>>).map((c) => String(c.id)),
  );
  for (const id of requiredForbiddenIds) {
    assertCondition(forbiddenIds.has(id), `missing required forbidden class ${id}`);
  }
  for (const cls of forbidden as Array<Record<string, unknown>>) {
    assertCondition(typeof cls.description === "string" && cls.description.length > 0, `forbidden class ${String(cls.id)} missing description`);
    for (const key of ["npm_package_names", "cargo_crate_name_substrings", "source_scan_patterns"]) {
      assertCondition(Array.isArray(cls[key]), `forbidden class ${String(cls.id)} missing ${key}`);
    }
  }

  const bundled = d.bundled_libraries;
  assertCondition(Array.isArray(bundled) && bundled.length > 0, "bundled_libraries empty");
  for (const lib of bundled as Array<Record<string, unknown>>) {
    assertCondition(typeof lib.family === "string" && lib.family.length > 0, "bundled library missing family");
    assertCondition(lib.ecosystem === "npm" || lib.ecosystem === "cargo", `bundled library ${String(lib.family)} has invalid ecosystem`);
    assertCondition(
      Array.isArray(lib.package_patterns) && lib.package_patterns.length > 0,
      `bundled library ${String(lib.family)} missing package_patterns`,
    );
    assertCondition(
      Array.isArray(lib.allowed_licenses) && lib.allowed_licenses.length > 0,
      `bundled library ${String(lib.family)} missing allowed_licenses`,
    );
  }

  assertCondition(Array.isArray(d.docker_opt_in_exceptions), "docker_opt_in_exceptions missing");
  assertCondition(
    Array.isArray(d.product_scan_roots) && d.product_scan_roots.length > 0,
    "product_scan_roots empty",
  );
  const manifests = d.product_manifests as Record<string, unknown> | undefined;
  assertCondition(typeof manifests === "object" && manifests !== null, "product_manifests missing");
  for (const key of ["npm", "npm_lockfiles", "cargo", "cargo_lockfiles"]) {
    assertCondition(
      Array.isArray(manifests[key]) && (manifests[key] as unknown[]).length > 0,
      `product_manifests.${key} empty`,
    );
  }

  return doc as RuntimeDependencyAllowlist;
}

/** The validated allowlist. Import-time validation: drift fails every consumer. */
export const RUNTIME_DEPENDENCY_ALLOWLIST: RuntimeDependencyAllowlist =
  validateAllowlistDocument(rawAllowlist);

/** True when `packageName` matches an exact pattern or a `prefix*` pattern. */
export function matchesPackagePattern(packageName: string, pattern: string): boolean {
  if (pattern.endsWith("*")) {
    return packageName.startsWith(pattern.slice(0, -1));
  }
  return packageName === pattern;
}

/** Returns the bundled-library rule covering an npm package name, if any. */
export function bundledNpmRuleFor(packageName: string): BundledLibraryRule | null {
  for (const rule of RUNTIME_DEPENDENCY_ALLOWLIST.bundled_libraries) {
    if (rule.ecosystem !== "npm") continue;
    if (rule.package_patterns.some((p) => matchesPackagePattern(packageName, p))) {
      return rule;
    }
  }
  return null;
}

/** True when the npm package belongs to the WP-009 editor stack (bundled-library families). */
export function isEditorStackNpmPackage(packageName: string): boolean {
  return bundledNpmRuleFor(packageName) !== null;
}

export function forbiddenClassById(id: string): ForbiddenRuntimeDependencyClass {
  const cls = RUNTIME_DEPENDENCY_ALLOWLIST.forbidden_runtime_dependency_classes.find(
    (c) => c.id === id,
  );
  if (!cls) throw new AllowlistShapeError(`forbidden class ${id} not found`);
  return cls;
}

/** All exact npm package names forbidden across every class. */
export function forbiddenNpmPackageNames(): readonly string[] {
  return RUNTIME_DEPENDENCY_ALLOWLIST.forbidden_runtime_dependency_classes.flatMap(
    (c) => c.npm_package_names,
  );
}

/** All cargo crate-name substrings forbidden across every class. */
export function forbiddenCargoCrateSubstrings(): readonly string[] {
  return RUNTIME_DEPENDENCY_ALLOWLIST.forbidden_runtime_dependency_classes.flatMap(
    (c) => c.cargo_crate_name_substrings,
  );
}

/** CDN host denylist used by source and built-bundle scans (MT-018 / MT-027). */
export function cdnHostDenylist(): readonly string[] {
  return forbiddenClassById("cdn_runtime_asset").source_scan_patterns;
}

/** Classifies a file path into an allowed external runtime input kind, if any. */
export function classifyExternalRuntimeInputPath(
  filePath: string,
): ExternalRuntimeInputKind | null {
  const lower = filePath.toLowerCase();
  for (const input of RUNTIME_DEPENDENCY_ALLOWLIST.allowed_external_runtime_inputs) {
    if (input.allowed_extensions.some((ext) => lower.endsWith(ext))) {
      return input.kind;
    }
  }
  return null;
}
