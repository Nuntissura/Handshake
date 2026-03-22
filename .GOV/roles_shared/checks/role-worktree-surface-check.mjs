import { execFileSync } from "node:child_process";
import { isGovernanceOnlyPath, normalizeRepoPath } from "../scripts/lib/scope-surface-lib.mjs";

function gitTrim(args) {
  try {
    return execFileSync("git", args, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "";
  }
}

function gitList(args) {
  const output = gitTrim(args);
  return output ? output.split(/\r?\n/).map((line) => normalizeRepoPath(line)).filter(Boolean) : [];
}

const branch = gitTrim(["rev-parse", "--abbrev-ref", "HEAD"]);
const governanceOnlyBranches = new Set(["gov_kernel", "role_orchestrator"]);

if (!governanceOnlyBranches.has(branch)) {
  console.log("role-worktree-surface-check skipped");
  process.exit(0);
}

const tracked = gitList(["diff", "--name-only", "HEAD"]);
const untracked = gitList(["ls-files", "--others", "--exclude-standard"]);
const changed = Array.from(new Set([...tracked, ...untracked]));
const violations = changed.filter((filePath) => !isGovernanceOnlyPath(filePath));

if (violations.length > 0) {
  console.error("[ROLE_WORKTREE_SURFACE_CHECK] Governance-only worktree contains non-governance edits.");
  for (const violation of violations) {
    console.error(`  - ${violation}`);
  }
  process.exit(1);
}

console.log("role-worktree-surface-check ok");
