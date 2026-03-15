import fs from "node:fs";
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

  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
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

function toPosix(value) {
  return value.split(path.sep).join("/");
}

const repoRoot = path.resolve(resolveRepoRoot());
process.chdir(repoRoot);

const roots = [
  path.join(repoRoot, "Handshake Codex v1.4.md"),
  path.join(repoRoot, "AGENTS.md"),
  path.join(repoRoot, "justfile"),
  path.join(repoRoot, ".github"),
  path.join(repoRoot, ".GOV", "README.md"),
  path.join(repoRoot, ".GOV", "GOV_KERNEL", "01_AUTHORITY_AND_ROLES.md"),
  path.join(repoRoot, ".GOV", "GOV_KERNEL", "02_ARTIFACTS_AND_CONTRACTS.md"),
  path.join(repoRoot, ".GOV", "GOV_KERNEL", "03_GATES_AND_ENFORCERS.md"),
  path.join(repoRoot, ".GOV", "GOV_KERNEL", "05_CI_HOOKS_AND_CONFIG.md"),
  path.join(repoRoot, ".GOV", "GOV_KERNEL", "06_VERSIONING_AND_DRIFT_CONTROL.md"),
  path.join(repoRoot, ".GOV", "roles", "README.md"),
  path.join(repoRoot, ".GOV", "roles", "STRUCTURE_RULES.md"),
  path.join(repoRoot, ".GOV", "roles", "coder", "README.md"),
  path.join(repoRoot, ".GOV", "roles", "coder", "CODER_PROTOCOL.md"),
  path.join(repoRoot, ".GOV", "roles", "coder", "checks"),
  path.join(repoRoot, ".GOV", "roles", "coder", "scripts"),
  path.join(repoRoot, ".GOV", "roles", "orchestrator", "README.md"),
  path.join(repoRoot, ".GOV", "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
  path.join(repoRoot, ".GOV", "roles", "orchestrator", "checks"),
  path.join(repoRoot, ".GOV", "roles", "orchestrator", "scripts"),
  path.join(repoRoot, ".GOV", "roles", "validator", "README.md"),
  path.join(repoRoot, ".GOV", "roles", "validator", "VALIDATOR_PROTOCOL.md"),
  path.join(repoRoot, ".GOV", "roles", "validator", "checks"),
  path.join(repoRoot, ".GOV", "roles", "validator", "scripts"),
  path.join(repoRoot, ".GOV", "roles_shared", "README.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "ARCHITECTURE.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "BOUNDARY_RULES.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "REPO_RESILIENCE.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "ROLE_SESSION_ORCHESTRATION.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "ROLE_WORKFLOW_QUICKREF.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "ROLE_WORKTREES.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "START_HERE.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "STRUCTURE_RULES.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "DEPRECATION_SUNSET_PLAN.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "VALIDATOR_FILE_TOUCH_MAP.md"),
  path.join(repoRoot, ".GOV", "roles_shared", "checks"),
  path.join(repoRoot, ".GOV", "roles_shared", "scripts"),
  path.join(repoRoot, ".GOV", "roles_shared", "exports"),
  path.join(repoRoot, ".GOV", "reference", "README.md"),
  path.join(repoRoot, ".GOV", "templates"),
  path.join(repoRoot, ".GOV", "schemas"),
  path.join(repoRoot, ".GOV", "tools"),
  path.join(repoRoot, ".GOV", "docs", "vscode-session-bridge", "GOVERNED_SESSION_CONTROL_ARCHITECTURE.md"),
];

const files = roots.flatMap((root) => {
  if (path.extname(root)) return [root];
  return listFilesRecursive(root);
});

const compatHintRe = /(legacy|deprecated|retired|compatibility|compat-only|compatibility-only|archive-only|derived evidence|do not add new dependencies)/i;
const staleChecks = [
  {
    label: "retired root .GOV/scripts surface",
    regex: /\.GOV\/scripts\//,
  },
  {
    label: "stale validator gate state path",
    regex: /\.GOV\/validator_gates\//,
  },
  {
    label: "stale legacy validator gate script path",
    regex: /\.GOV\/scripts\/validation\/validator_gates\.mjs/,
  },
  {
    label: "deprecated top-level ROLE_MAILBOX authority path",
    regex: /\.GOV\/ROLE_MAILBOX\//,
  },
];

const violations = [];

for (const filePath of files) {
  const rel = toPosix(path.relative(repoRoot, filePath));
  if (
    rel.startsWith(".GOV/task_packets/") ||
    rel.startsWith(".GOV/refinements/") ||
    rel.startsWith(".GOV/Audits/") ||
    rel.startsWith(".GOV/operator/") ||
    rel.startsWith(".GOV/roles_shared/WP_COMMUNICATIONS/") ||
    rel.startsWith(".GOV/roles_shared/SESSION_CONTROL_OUTPUTS/")
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
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (compatHintRe.test(line)) continue;
    for (const check of staleChecks) {
      if (check.regex.test(line)) {
        violations.push(`${rel}:${index + 1}: ${check.label}: ${line}`);
      }
    }
  }
}

if (violations.length > 0) {
  console.error("migration-path-truth-check: FAIL - stale active governance path references found.");
  console.error(violations.join("\n"));
  process.exit(1);
}

console.log("migration-path-truth-check ok");
