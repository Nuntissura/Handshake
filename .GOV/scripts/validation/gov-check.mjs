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

  // This file lives at: /.GOV/scripts/validation/gov-check.mjs
  // Up 3 => repo root.
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

const repoRoot = path.resolve(resolveRepoRoot());
process.chdir(repoRoot);

// Governance-only checks (no product source scanning).
await import("../spec-current-check.mjs");
await import("./atelier_role_registry_check.mjs");
await import("./task-board-check.mjs");
await import("./task-packet-claim-check.mjs");
await import("./wp-activation-traceability-check.mjs");
await import("./worktree-concurrency-check.mjs");
await import("./lifecycle-ux-check.mjs");
await import("./drive-agnostic-check.mjs");
await import("./phase1-add-coverage-check.mjs");

console.log("gov-check ok");
