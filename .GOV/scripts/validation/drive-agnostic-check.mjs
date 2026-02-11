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

  // This file lives at: /.GOV/scripts/validation/drive-agnostic-check.mjs
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

const DRIVE_PATH_RE = /\b[A-Za-z]:[\\/]/;
// Matches both literal UNC paths (example: `\\server\share`) and common escaped representations in code (example: `\\\\server\\share`).
const UNC_PATH_RE =
  /\\\\[A-Za-z0-9._-]+\\[A-Za-z0-9.$_-]+|\\\\\\\\[A-Za-z0-9._-]+\\\\[A-Za-z0-9.$_-]+/;
const EXAMPLE_HINT_RE = /\bexample\b/i;

// Intent: keep repo governance drive-agnostic. Evidence logs / historical task packets are excluded.
const roots = [
  path.join(repoRoot, "Handshake Codex v1.4.md"),
  path.join(repoRoot, "AGENTS.md"),
  path.join(repoRoot, ".GOV", "roles"),
  path.join(repoRoot, ".GOV", "roles_shared"),
  path.join(repoRoot, ".GOV", "scripts"),
  path.join(repoRoot, ".github"),
  path.join(repoRoot, "justfile"),
];

const files = roots.flatMap((root) => {
  if (path.extname(root)) return [root];
  return listFilesRecursive(root);
});

const violations = [];

for (const filePath of files) {
  // Exclude evidence/history folders that may contain old absolute paths.
  const rel = toPosix(path.relative(repoRoot, filePath));
  if (
    rel.startsWith(".GOV/task_packets/") ||
    rel.startsWith(".GOV/refinements/") ||
    rel.startsWith(".GOV/operator/")
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
    if (!DRIVE_PATH_RE.test(line) && !UNC_PATH_RE.test(line)) continue;

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
