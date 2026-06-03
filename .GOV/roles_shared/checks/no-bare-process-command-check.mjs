import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const CHECK_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_REPO_ROOT = path.resolve(CHECK_DIR, "../../..");
const TARGET_PATHS = [
  "src/backend/handshake_core/src/model_runtime",
  "src/backend/handshake_core/src/kernel/model_adapter.rs",
];
const CODE_EXTENSIONS = new Set([".rs"]);
const SKIP_DIRS = new Set([".git", ".GOV", "target", "node_modules"]);
const FORBIDDEN = [
  {
    token: "Command::new",
    pattern: /\bCommand\s*::\s*new\b/g,
  },
  {
    token: "std::process::Command",
    pattern: /\bstd\s*::\s*process\s*::\s*Command\b/g,
  },
  {
    token: "tokio::process::Command",
    pattern: /\btokio\s*::\s*process\s*::\s*Command\b/g,
  },
];

function parseArgs(argv) {
  const args = {
    repoRoot: "",
    help: false,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (arg === "--repo-root") {
      args.repoRoot = String(argv[i + 1] || "").trim();
      i += 1;
      continue;
    }
    if (!arg.startsWith("-") && !args.repoRoot) {
      args.repoRoot = arg;
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

function usage() {
  return [
    "Usage: node .GOV/roles_shared/checks/no-bare-process-command-check.mjs [--repo-root <path>]",
    "",
    "Scans model runtime process-spawn surfaces for direct std/tokio Command usage.",
    "Violations are emitted as JSONL to stdout and exit with code 2.",
  ].join("\n");
}

function resolveRepoRoot(repoRootArg) {
  if (repoRootArg) return path.resolve(repoRootArg);
  const envRoot = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  if (envRoot) return path.resolve(envRoot);
  const cwd = path.resolve(process.cwd());
  if (TARGET_PATHS.some((targetPath) => fs.existsSync(path.join(cwd, targetPath)))) {
    return cwd;
  }
  try {
    const out = execFileSync("git", ["-C", DEFAULT_REPO_ROOT, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return path.resolve(out);
  } catch {
    // Fall back to file-relative repository root.
  }
  return DEFAULT_REPO_ROOT;
}

function normalizeRelative(filePath, repoRoot) {
  return path.relative(repoRoot, filePath).replace(/\\/g, "/");
}

function shouldScanFile(filePath) {
  return CODE_EXTENSIONS.has(path.extname(filePath).toLowerCase());
}

function walkFiles(rootPath) {
  if (!fs.existsSync(rootPath)) return [];
  const stat = fs.statSync(rootPath);
  if (stat.isFile()) return shouldScanFile(rootPath) ? [rootPath] : [];
  const files = [];
  const stack = [rootPath];
  while (stack.length > 0) {
    const current = stack.pop();
    const entries = fs.readdirSync(current, { withFileTypes: true });
    for (const entry of entries) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        if (!SKIP_DIRS.has(entry.name)) stack.push(entryPath);
        continue;
      }
      if (entry.isFile() && shouldScanFile(entryPath)) files.push(entryPath);
    }
  }
  return files.sort((a, b) => a.localeCompare(b));
}

function lineAndColumn(text, index) {
  const prefix = text.slice(0, index);
  const lines = prefix.split(/\r?\n/);
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1,
  };
}

function scanFile(filePath, repoRoot) {
  const source = fs.readFileSync(filePath, "utf8");
  const violations = [];
  for (const forbidden of FORBIDDEN) {
    forbidden.pattern.lastIndex = 0;
    for (const match of source.matchAll(forbidden.pattern)) {
      const location = lineAndColumn(source, match.index || 0);
      violations.push({
        check_id: "NO_BARE_PROCESS_COMMAND",
        reason: "Model runtime process launches must route through SandboxAdapter::spawn.",
        file: normalizeRelative(filePath, repoRoot),
        line: location.line,
        column: location.column,
        token: forbidden.token,
      });
    }
  }
  return violations;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log(usage());
    return;
  }
  const repoRoot = resolveRepoRoot(args.repoRoot);
  const files = TARGET_PATHS.flatMap((targetPath) => walkFiles(path.join(repoRoot, targetPath)));
  const violations = files.flatMap((filePath) => scanFile(filePath, repoRoot));
  if (violations.length > 0) {
    for (const violation of violations) console.log(JSON.stringify(violation));
    process.exitCode = 2;
    return;
  }
  console.log(JSON.stringify({
    check_id: "NO_BARE_PROCESS_COMMAND",
    status: "PASS",
    scanned_files: files.map((filePath) => normalizeRelative(filePath, repoRoot)),
  }));
}

main();
