import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

function resolveRepoRoot() {
  const injectedRepoRoot = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  if (injectedRepoRoot) {
    return injectedRepoRoot;
  }

  const fileRelativeRepoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  try {
    const out = execFileSync("git", ["-C", fileRelativeRepoRoot, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore; fall back to relative-to-file resolution.
  }

  // This file lives at: /.GOV/roles_shared/checks/gov-check.mjs
  // Up 3 => repo root.
  return fileRelativeRepoRoot;
}

const repoRoot = path.resolve(resolveRepoRoot());
if (!String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim()) {
  process.env.HANDSHAKE_ACTIVE_REPO_ROOT = repoRoot;
}
process.chdir(repoRoot);

// Governance-only checks (no product source scanning).
// Bundled checks [RGF-194] run sub-checks as child processes and collect ALL failures.
// 6 bundles replace 24 individual imports; 8 standalone checks remain (unique purpose, no natural grouping).
await import("../scripts/spec-current-check.mjs");
await import("./spec-bundle-check.mjs");                       // spec-debt-registry, spec-eof-appendices, spec-growth-discipline, spec-governance-reference
await import("./atelier_role_registry_check.mjs");
await import("../../roles/validator/checks/validator-report-structure-check.mjs");
await import("./packet-truth-bundle-check.mjs");               // task-board, packet-closure-monitor, packet-truth, merge-progression-truth, task-packet-claim
await import("./semantic-proof-check.mjs");
await import("./computed-policy-gate-check.mjs");
await import("./historical-smoketest-lineage-check.mjs");
await import("./build-order-check.mjs");
await import("./wp-comm-bundle-check.mjs");                    // wp-communications, wp-activation-traceability
await import("./session-bundle-check.mjs");                    // session-policy, session-launch-runtime, session-control-runtime, lifecycle-ux
await import("./governance-structure-bundle-check.mjs");       // protocol-alignment, prevention-ladder, deprecation-sunset, migration-path-truth
await import("./topology-bundle-check.mjs");                   // worktree-concurrency, drive-agnostic, runtime-placement, role-worktree-surface, topology-registry
await import("./phase1-add-coverage-check.mjs");
await import("./memory-health-check.mjs");

// Lightweight memory maintenance — runs dedup if >6h stale, full compact if >24h stale.
// Safe on every gov-check: staleness gates prevent redundant work.
try {
  const memFs = await import("node:fs");
  const memPath = await import("node:path");
  const { GOVERNANCE_RUNTIME_ROOT_ABS } = await import("../scripts/lib/runtime-paths.mjs");
  const dbPath = memPath.default.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "GOVERNANCE_MEMORY.db");
  if (memFs.default.existsSync(dbPath)) {
    const { DatabaseSync } = await import("node:sqlite");
    const db = new DatabaseSync(dbPath, { readOnly: true });
    try {
      const last = db.prepare("SELECT run_at FROM consolidation_log ORDER BY run_at DESC LIMIT 1").get();
      const sinceMs = last ? Date.now() - new Date(last.run_at).getTime() : Infinity;
      db.close();
      if (sinceMs > 6 * 60 * 60 * 1000) {
        const { execFileSync } = await import("node:child_process");
        const scriptPath = memPath.default.join(memPath.default.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1")), "..", "scripts", "memory", "memory-compact.mjs");
        try {
          execFileSync(process.execPath, [scriptPath], { stdio: "ignore" });
          console.log("memory-maintenance ok (compaction ran)");
        } catch {
          console.log("memory-maintenance ok (compaction attempted, non-fatal)");
        }
      }
    } catch { try { db.close(); } catch {} }
  }
} catch { /* memory maintenance is best-effort */ }

console.log("gov-check ok");
