import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";

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

  // This file lives at: /.GOV/roles_shared/checks/drive-agnostic-check.mjs
  // Up 3 => repo root.
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

const repoRoot = path.resolve(resolveRepoRoot());
process.chdir(repoRoot);

function fail(message, details = "") {
  console.error(message);
  if (details) console.error(details);
  process.exit(1);
}

function listFilesRecursive(rootDir) {
  const out = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const current = stack.pop();
    if (!current) continue;

    let entries;
    try {
      entries = fs.readdirSync(current, { withFileTypes: true });
    } catch {
      continue;
    }

    for (const entry of entries) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else if (entry.isFile()) {
        out.push(full);
      }
    }
  }
  return out;
}

function toPosix(p) {
  return p.split(path.sep).join("/");
}

// Split Windows drive detection so slash-style paths require at least two segments.
// This avoids regex-literal false positives while still catching real multi-segment host paths.
const DRIVE_BACKSLASH_RE = /\b[A-Za-z]:\\/;
const DRIVE_MULTI_SEGMENT_SLASH_RE = /\b[A-Za-z]:\/[A-Za-z0-9._-]+\/[^\s"'`]+/;
// Matches both literal UNC paths (example: `\\server\share`) and common escaped representations in code (example: `\\\\server\\share`).
const UNC_PATH_RE =
  /\\\\[A-Za-z0-9._-]+\\[A-Za-z0-9.$_-]+|\\\\\\\\[A-Za-z0-9._-]+\\\\[A-Za-z0-9.$_-]+/;
// Catch common absolute host roots while avoiding generic repo-relative slash paths.
const POSIX_HOST_PATH_RE = /\/(mnt|home|Users|Volumes|workspace|tmp|var\/tmp|opt|srv)\/[^\s"'`]+/;
const EXAMPLE_HINT_RE = /\bexample\b/i;

// Intent: keep active governance authority surfaces drive-agnostic. Historical evidence surfaces are excluded.
const roots = [
  path.join(repoRoot, "Handshake Codex v1.4.md"),
  path.join(repoRoot, "AGENTS.md"),
  path.join(repoRoot, "justfile"),
  path.join(repoRoot, ".github"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "README.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "GOV_KERNEL", "01_AUTHORITY_AND_ROLES.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "GOV_KERNEL", "02_ARTIFACTS_AND_CONTRACTS.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "GOV_KERNEL", "03_GATES_AND_ENFORCERS.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "GOV_KERNEL", "05_CI_HOOKS_AND_CONFIG.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "GOV_KERNEL", "06_VERSIONING_AND_DRIFT_CONTROL.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "README.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "coder", "README.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "coder", "CODER_PROTOCOL.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "coder", "checks"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "coder", "scripts"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "orchestrator", "README.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "orchestrator", "checks"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "validator", "README.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "validator", "VALIDATOR_PROTOCOL.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "validator", "checks"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles", "validator", "scripts"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "README.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "ARCHITECTURE.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "BOUNDARY_RULES.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "REPO_RESILIENCE.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "ROLE_SESSION_ORCHESTRATION.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "ROLE_WORKFLOW_QUICKREF.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "ROLE_WORKTREES.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "START_HERE.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "DEPRECATION_SUNSET_PLAN.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "docs", "VALIDATOR_FILE_TOUCH_MAP.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "records"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "checks"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "scripts"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "exports"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "roles_shared", "schemas"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "reference", "README.md"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "templates"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "tools"),
  path.join(repoRoot, GOV_ROOT_REPO_REL, "docs", "vscode-session-bridge", "GOVERNED_SESSION_CONTROL_ARCHITECTURE.md"),
];

const files = roots.flatMap((root) => {
  if (path.extname(root)) return [root];
  return listFilesRecursive(root);
});

const violations = [];

function isExcludedRuntimeArtifact(relPath) {
  return relPath === `${GOV_ROOT_REPO_REL}/roles_shared/runtime/ROLE_SESSION_REGISTRY.json`
    || relPath === `${GOV_ROOT_REPO_REL}/roles_shared/runtime/SESSION_LAUNCH_REQUESTS.jsonl`
    || relPath === `${GOV_ROOT_REPO_REL}/roles_shared/runtime/SESSION_CONTROL_REQUESTS.jsonl`
    || relPath === `${GOV_ROOT_REPO_REL}/roles_shared/runtime/SESSION_CONTROL_RESULTS.jsonl`
    || relPath === `${GOV_ROOT_REPO_REL}/roles_shared/runtime/SESSION_CONTROL_BROKER_STATE.json`
    || relPath.startsWith(`${GOV_ROOT_REPO_REL}/roles_shared/runtime/SESSION_CONTROL_OUTPUTS/`);
}

for (const filePath of files) {
  // Exclude evidence/history folders that may contain old absolute paths.
  const rel = toPosix(path.relative(repoRoot, filePath));
  if (
    rel.startsWith(`${GOV_ROOT_REPO_REL}/task_packets/`) ||
    rel.startsWith(`${GOV_ROOT_REPO_REL}/refinements/`) ||
    rel.startsWith(`${GOV_ROOT_REPO_REL}/Audits/`) ||
    rel.startsWith(`${GOV_ROOT_REPO_REL}/operator/`) ||
    isExcludedRuntimeArtifact(rel)
  ) {
    continue;
  }

  let content;
  try {
    content = fs.readFileSync(filePath, "utf8");
  } catch {
    continue;
  }

  const lines = content.split(/\r?\n/);
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (
      !DRIVE_BACKSLASH_RE.test(line)
      && !DRIVE_MULTI_SEGMENT_SLASH_RE.test(line)
      && !UNC_PATH_RE.test(line)
      && !POSIX_HOST_PATH_RE.test(line)
    ) continue;

    // Allow drive letters only in explicitly-marked examples to prevent policy drift.
    if (EXAMPLE_HINT_RE.test(line)) continue;

    violations.push(`${rel}:${i + 1}:${line}`);
  }
}

if (violations.length > 0) {
  fail(
    "drive-agnostic-check: FAIL - drive-letter or UNC absolute paths found in governance surface (non-example).",
    violations.join("\n"),
  );
}

console.log("drive-agnostic-check ok");
