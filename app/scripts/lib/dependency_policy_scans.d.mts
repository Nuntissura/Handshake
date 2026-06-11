// Type declarations for dependency_policy_scans.mjs (consumed by vitest TS tests).

export declare const ALLOWLIST_RELATIVE_PATH: string;

export interface AllowlistDocument {
  schema: string;
  version: string;
  wp_id: string;
  mt_id: string;
  allowed_external_runtime_inputs: Array<{
    kind: string;
    operator_gated: boolean;
    default_enabled: boolean;
    allowed_extensions: string[];
    owning_surface: string;
  }>;
  forbidden_runtime_dependency_classes: Array<{
    id: string;
    description: string;
    npm_package_names: string[];
    cargo_crate_name_substrings: string[];
    source_scan_patterns: string[];
  }>;
  bundled_libraries: Array<{
    family: string;
    ecosystem: "npm" | "cargo";
    package_patterns: string[];
    allowed_licenses: string[];
    reason: string;
  }>;
  docker_opt_in_exceptions: Array<{ path_prefix: string; reason: string }>;
  product_scan_roots: string[];
  product_manifests: {
    npm: string[];
    npm_lockfiles: string[];
    cargo: string[];
    cargo_lockfiles: string[];
  };
}

export interface PatternViolation {
  path: string;
  line: number;
  pattern: string;
  snippet: string;
}

export interface PatternScanResult {
  violations: PatternViolation[];
  exceptionsApplied: Array<{ path: string; exception: string; patterns: string[] }>;
}

export declare function loadAllowlist(repoRoot: string): AllowlistDocument;
export declare function walkSourceFiles(rootDir: string): string[];
export declare function scanFilesForPatterns(args: {
  repoRoot: string;
  files: string[];
  patterns: string[];
  exceptPathPrefixes?: string[];
  excludePathSubstrings?: string[];
}): PatternScanResult;
export declare function scanDockerDefault(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): PatternScanResult;
export declare function scanCdnReferences(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): PatternScanResult;
export declare function npmManifestDependencyNames(packageJsonText: string): string[];
export declare function cargoManifestDependencyNames(cargoTomlText: string): string[];
export declare function cargoLockPackageNames(cargoLockText: string): string[];
export declare function pnpmLockPackageNames(pnpmLockText: string): string[];
export declare function scanForbiddenManifestPackages(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): {
  violations: Array<{ class: string; ecosystem: string; manifest: string; package: string }>;
};
export declare function scanCargoLockUnionEntries(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): { advisories: Array<{ class: string; lockfile: string; package: string }> };
export declare function cargoTreeProvesAbsent(args: {
  manifestDir: string;
  crateName: string;
}): boolean;
export declare function auditPnpmLockSync(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): {
  issues: Array<{
    kind: string;
    section?: string;
    package?: string;
    manifest?: string;
    lock?: string;
    detail?: string;
  }>;
};
export declare function auditEditorStackResolution(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): {
  violations: Array<{ package: string; problem: string }>;
  audited: Array<{ package: string; version: string }>;
};
export declare function auditEditorStackLicenses(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
  appDir: string;
}): {
  violations: Array<{ package: string; problem: string }>;
  inventory: Array<{ package: string; version: string; license: string; family: string }>;
};
export declare function repoRootFromScriptsLib(importMetaUrl: string): string;
export declare function pathExists(p: string): boolean;
export declare function mtimeMs(p: string): number;
