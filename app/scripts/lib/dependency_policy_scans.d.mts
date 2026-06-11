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
  built_output_scan_exceptions?: Array<{
    pattern: string;
    required_context_marker: string;
    max_marker_distance: number;
    forward_only?: boolean;
    max_total_occurrences?: number;
    dependency: string;
    reason: string;
    mt: string;
  }>;
  product_scan_roots: string[];
  product_manifests: {
    npm: string[];
    npm_lockfiles: string[];
    cargo: string[];
    cargo_lockfiles: string[];
  };
  scan_self_exempt_paths: { description?: string; paths: string[] };
  docker_artifact_scan: {
    description?: string;
    filename_globs: string[];
    shell_extensions?: string[];
    shell_content_markers?: string[];
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
export declare function selfExemptPathSet(allowlist: AllowlistDocument): Set<string>;
export declare function walkSourceFiles(rootDir: string): string[];
export declare function walkAllFiles(rootDir: string): string[];
export declare function globMatchesFilename(glob: string, filename: string): boolean;
export declare function scanFilesForPatterns(args: {
  repoRoot: string;
  files: string[];
  patterns: string[];
  exceptPathPrefixes?: string[];
  excludePathSubstrings?: string[];
  exactExemptPaths?: Set<string> | string[] | null;
}): PatternScanResult;
export declare function scanDockerDefault(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): PatternScanResult;
export declare function scanDockerArtifacts(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): {
  violations: Array<{ path: string; reason: string }>;
  exceptionsApplied: Array<{ path: string; reason: string; exception: string }>;
};
export declare function scanCdnReferences(args: {
  repoRoot: string;
  allowlist: AllowlistDocument;
}): PatternScanResult;
export declare function npmManifestDependencyNames(packageJsonText: string): string[];
export declare function externalWorkerLoads(
  content: string,
  path: string,
): Array<{ path: string; site: string; url: string }>;
export declare function partitionCdnHits(args: {
  content: string;
  relPath: string;
  pattern: string;
  allowlist: AllowlistDocument;
}): {
  violations: Array<{ path: string; pattern: string; offset: number }>;
  exempted: Array<{ path: string; pattern: string; dependency: string; marker: string }>;
};
export declare function assertSingleOccurrenceExceptions(args: {
  files: Array<{ relPath: string; content: string }>;
  allowlist: AllowlistDocument;
}): {
  perPattern: Array<{
    pattern: string;
    count: number;
    max: number;
    ok: boolean;
    locations: Array<{ path: string; count: number }>;
  }>;
  violations: Array<{
    pattern: string;
    count: number;
    max: number;
    dependency: string;
    locations: Array<{ path: string; count: number }>;
    detail: string;
  }>;
};
export declare function normalizeSplitHostLiterals(content: string): string;
export declare function scanSplitHostCdn(args: {
  content: string;
  relPath: string;
  patterns: string[];
}): Array<{ path: string; pattern: string; kind: string }>;
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
