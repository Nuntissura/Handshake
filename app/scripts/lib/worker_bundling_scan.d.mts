import type { AllowlistDocument } from "./dependency_policy_scans.mjs";

export declare const REQUIRED_MONACO_WORKERS: string[];

export interface WorkerBundleTree {
  dist: string;
  files_scanned: number;
  bundles_monaco: boolean;
  worker_chunks: string[];
  missing_monaco_workers: string[];
  external_worker_refs: Array<{ path: string; site: string; url: string }>;
  cdn_hits: Array<{ path: string; pattern: string; offset: number }>;
  cdn_exceptions_applied: Array<{ path: string; pattern: string; dependency: string; marker: string }>;
  split_host_cdn_hits: Array<{ path: string; pattern: string; kind: string }>;
  occurrence_caps: Array<{
    pattern: string;
    count: number;
    max: number;
    ok: boolean;
    locations: Array<{ path: string; count: number }>;
  }>;
  occurrence_violations: Array<{
    pattern: string;
    count: number;
    max: number;
    dependency: string;
    locations: Array<{ path: string; count: number }>;
    detail: string;
  }>;
}

export declare function listFilesRecursive(rootDir: string, skipNames?: Set<string>): string[];
export declare function relativeReportPath(reportRoot: string, fullPath: string): string;
export declare function scanWorkerBundleTree(
  distDir: string,
  allowlist: AllowlistDocument,
  options?: { reportRoot?: string },
): WorkerBundleTree;
