import { existsSync, readFileSync, readdirSync } from "node:fs";
import { relative, sep } from "node:path";
import {
  assertSingleOccurrenceExceptions,
  externalWorkerLoads,
  partitionCdnHits,
  scanSplitHostCdn,
} from "./dependency_policy_scans.mjs";

export const REQUIRED_MONACO_WORKERS = ["editor", "ts", "json", "css", "html"];

export function listFilesRecursive(rootDir, skipNames = new Set()) {
  if (!existsSync(rootDir)) return [];
  const out = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const dir = stack.pop();
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (skipNames.has(entry.name)) continue;
      const full = `${dir}${sep}${entry.name}`;
      if (entry.isDirectory()) stack.push(full);
      else out.push(full);
    }
  }
  return out;
}

function slashPath(value) {
  return value.split(sep).join("/");
}

export function relativeReportPath(reportRoot, fullPath) {
  const relPath = slashPath(relative(reportRoot, fullPath));
  return relPath.length > 0 ? relPath : ".";
}

export function scanWorkerBundleTree(distDir, allowlist, options = {}) {
  const reportRoot = options.reportRoot || distDir;
  const cdnPatterns = allowlist.forbidden_runtime_dependency_classes
    .find((c) => c.id === "cdn_runtime_asset")
    .source_scan_patterns.map((p) => p.toLowerCase());

  const files = listFilesRecursive(distDir);
  const jsFiles = files.filter((f) => /\.(js|mjs|cjs)$/i.test(f));
  const textFiles = files.filter((f) => /\.(js|mjs|cjs|html|css|json|webmanifest)$/i.test(f));

  const externalWorkerRefs = [];
  const cdnHits = [];
  const cdnExceptionsApplied = [];
  const splitHostCdnHits = [];
  const textContents = [];
  let monacoMarker = false;

  for (const file of textFiles) {
    const relPath = relativeReportPath(reportRoot, file);
    const content = readFileSync(file, "utf8");
    textContents.push({ relPath, content });
    for (const pattern of cdnPatterns) {
      const { violations, exempted } = partitionCdnHits({
        content,
        relPath,
        pattern,
        allowlist,
      });
      cdnHits.push(...violations);
      cdnExceptionsApplied.push(...exempted);
    }
    splitHostCdnHits.push(...scanSplitHostCdn({ content, relPath, patterns: cdnPatterns }));
    if (jsFiles.includes(file)) {
      externalWorkerRefs.push(...externalWorkerLoads(content, relPath));
      if (content.includes("MonacoEnvironment")) monacoMarker = true;
    }
  }

  const { perPattern: occurrenceCaps, violations: occurrenceViolations } =
    assertSingleOccurrenceExceptions({ files: textContents, allowlist });

  const workerChunks = files
    .map((f) => relativeReportPath(reportRoot, f))
    .filter((f) => /(^|\/)[^/]*\.worker-[^/]*\.js$/.test(f) || /(^|\/)worker-[^/]*\.js$/.test(f))
    .sort();

  const missingMonacoWorkers = monacoMarker
    ? REQUIRED_MONACO_WORKERS.filter(
        (kind) => !workerChunks.some((chunk) => chunk.includes(`${kind}.worker-`)),
      )
    : [];

  return {
    dist: relativeReportPath(reportRoot, distDir),
    files_scanned: textFiles.length,
    bundles_monaco: monacoMarker,
    worker_chunks: workerChunks,
    missing_monaco_workers: missingMonacoWorkers,
    external_worker_refs: externalWorkerRefs,
    cdn_hits: cdnHits,
    cdn_exceptions_applied: cdnExceptionsApplied,
    split_host_cdn_hits: splitHostCdnHits,
    occurrence_caps: occurrenceCaps,
    occurrence_violations: occurrenceViolations,
  };
}
