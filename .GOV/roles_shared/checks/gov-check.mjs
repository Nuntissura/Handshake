import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

function resolveRepoRoot() {
  try {
    const out = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore; fall back to relative-to-file resolution.
  }

  // This file lives at: /.GOV/roles_shared/checks/gov-check.mjs
  // Up 3 => repo root.
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

const repoRoot = path.resolve(resolveRepoRoot());
process.env.HANDSHAKE_ACTIVE_REPO_ROOT = repoRoot;
process.chdir(repoRoot);

// Governance-only checks (no product source scanning).
await import("../scripts/spec-current-check.mjs");
await import("./spec-eof-appendices-check.mjs");
await import("./spec-debt-registry-check.mjs");
await import("./atelier_role_registry_check.mjs");
await import("./task-board-check.mjs");
await import("../../roles/validator/checks/validator-report-structure-check.mjs");
await import("./packet-closure-monitor-check.mjs");
await import("./semantic-proof-check.mjs");
await import("./packet-truth-check.mjs");
await import("./wp-communications-check.mjs");
await import("./build-order-check.mjs");
await import("./task-packet-claim-check.mjs");
await import("./session-policy-check.mjs");
await import("./protocol-alignment-check.mjs");
await import("./session-launch-runtime-check.mjs");
await import("./session-control-runtime-check.mjs");
await import("./wp-activation-traceability-check.mjs");
await import("./worktree-concurrency-check.mjs");
await import("./lifecycle-ux-check.mjs");
await import("./drive-agnostic-check.mjs");
await import("./migration-path-truth-check.mjs");
await import("./phase1-add-coverage-check.mjs");
await import("./spec-growth-discipline-check.mjs");
await import("./spec-governance-reference-check.mjs");
await import("./deprecation-sunset-check.mjs");
await import("./topology-registry-check.mjs");

console.log("gov-check ok");
