#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const CHECK_DIR = path.dirname(fileURLToPath(import.meta.url));
const GOV_WORKTREE_ROOT = path.resolve(CHECK_DIR, "../../..");
const DEFAULT_PRODUCT_WORKTREE_ROOT = path.resolve(GOV_WORKTREE_ROOT, "../wtc-kernel-004-fold-v1");

const EXIT_OK = 0;
const EXIT_VIOLATION = 2;
const EXIT_USAGE_OR_MALFORMED = 3;

const PRODUCT_ROOTS = [
  "src/backend/handshake_core/src/kernel",
  "src/backend/handshake_core/src/storage",
];

const FORBIDDEN_PATTERNS = [
  "DocketAdapter",
  "docket_adapter",
  "DockerRunner",
  "docker_runner",
].map((pattern) => ({
  pattern,
  regex: new RegExp(`\\b${pattern}\\b`, "g"),
}));

const SKIP_DIRS = new Set([
  "test",
  "tests",
  "testdata",
  "fixture",
  "fixtures",
  "__fixtures__",
]);

function parseArgs(argv) {
  const args = {
    repoRoot: "",
    help: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (arg === "--repo-root") {
      args.repoRoot = String(argv[index + 1] || "").trim();
      index += 1;
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
    "Usage: node .GOV/roles_shared/checks/no-direct-docket-adapter-import-check.mjs [--repo-root <product-root>]",
    "",
    "Scans handshake_core kernel/storage Rust production roots for direct KERNEL-003 Docker/Docket identifiers.",
    "Violations are emitted as JSONL to stdout and exit with code 2.",
  ].join("\n");
}

function normalizePath(filePath) {
  return filePath.replace(/\\/g, "/");
}

function normalizeRelative(filePath, repoRoot) {
  return normalizePath(path.relative(repoRoot, filePath));
}

function productRootsExist(repoRoot) {
  return PRODUCT_ROOTS.every((relativeRoot) => {
    const absoluteRoot = path.join(repoRoot, relativeRoot);
    return fs.existsSync(absoluteRoot) && fs.statSync(absoluteRoot).isDirectory();
  });
}

function missingProductRoots(repoRoot) {
  return PRODUCT_ROOTS.filter((relativeRoot) => {
    const absoluteRoot = path.join(repoRoot, relativeRoot);
    return !fs.existsSync(absoluteRoot) || !fs.statSync(absoluteRoot).isDirectory();
  });
}

function resolveProductRoot(repoRootArg) {
  if (repoRootArg) return path.resolve(repoRootArg);

  const envRoot = String(process.env.HANDSHAKE_PRODUCT_REPO_ROOT || "").trim();
  if (envRoot) return path.resolve(envRoot);

  if (productRootsExist(GOV_WORKTREE_ROOT)) return GOV_WORKTREE_ROOT;

  return DEFAULT_PRODUCT_WORKTREE_ROOT;
}

function shouldSkipDirectory(name) {
  return SKIP_DIRS.has(name.toLowerCase());
}

function shouldScanFile(filePath) {
  return path.extname(filePath).toLowerCase() === ".rs";
}

function walkRustFiles(rootDir) {
  if (!fs.existsSync(rootDir)) return [];
  const files = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        if (!shouldSkipDirectory(entry.name)) stack.push(entryPath);
        continue;
      }
      if (entry.isFile() && shouldScanFile(entryPath)) {
        files.push(entryPath);
      }
    }
  }
  return files.sort((a, b) => a.localeCompare(b));
}

function rawStringStart(text, index) {
  let offset = 0;
  if (text[index] === "b" && text[index + 1] === "r") {
    offset = 2;
  } else if (text[index] === "r") {
    offset = 1;
  } else {
    return null;
  }

  let cursor = index + offset;
  let hashes = 0;
  while (text[cursor] === "#") {
    hashes += 1;
    cursor += 1;
  }
  if (text[cursor] !== '"') return null;

  const previous = text[index - 1] || "";
  if (/[A-Za-z0-9_]/.test(previous)) return null;
  return {
    prefixLength: offset + hashes + 1,
    terminator: `"${"#".repeat(hashes)}`,
  };
}

function maskSourceTrivia(text) {
  let output = "";
  let index = 0;
  let blockDepth = 0;
  let stringQuote = "";
  let rawTerminator = "";

  while (index < text.length) {
    const char = text[index];
    const next = text[index + 1] || "";

    if (char === "\r" || char === "\n") {
      output += char;
      index += 1;
      continue;
    }

    if (blockDepth > 0) {
      if (char === "/" && next === "*") {
        output += "  ";
        blockDepth += 1;
        index += 2;
      } else if (char === "*" && next === "/") {
        output += "  ";
        blockDepth -= 1;
        index += 2;
      } else {
        output += " ";
        index += 1;
      }
      continue;
    }

    if (rawTerminator) {
      if (text.startsWith(rawTerminator, index)) {
        output += " ".repeat(rawTerminator.length);
        index += rawTerminator.length;
        rawTerminator = "";
      } else {
        output += " ";
        index += 1;
      }
      continue;
    }

    if (stringQuote) {
      if (char === "\\" && stringQuote !== "`") {
        output += " ";
        if (index + 1 < text.length && next !== "\r" && next !== "\n") {
          output += " ";
          index += 2;
        } else {
          index += 1;
        }
      } else if (char === stringQuote) {
        output += " ";
        stringQuote = "";
        index += 1;
      } else {
        output += " ";
        index += 1;
      }
      continue;
    }

    if (char === "/" && next === "/") {
      output += "  ";
      index += 2;
      while (index < text.length && text[index] !== "\n" && text[index] !== "\r") {
        output += " ";
        index += 1;
      }
      continue;
    }

    if (char === "/" && next === "*") {
      output += "  ";
      blockDepth = 1;
      index += 2;
      continue;
    }

    const rawStart = rawStringStart(text, index);
    if (rawStart) {
      output += " ".repeat(rawStart.prefixLength);
      index += rawStart.prefixLength;
      rawTerminator = rawStart.terminator;
      continue;
    }

    if (char === '"' || char === "'" || char === "`") {
      output += " ";
      stringQuote = char;
      index += 1;
      continue;
    }

    output += char;
    index += 1;
  }

  return output;
}

function findViolationsInFile(filePath, repoRoot) {
  const source = fs.readFileSync(filePath, "utf8");
  const masked = maskSourceTrivia(source);
  const relativePath = normalizeRelative(filePath, repoRoot);
  const lines = masked.split(/\r?\n/);
  const violations = [];

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    for (const forbidden of FORBIDDEN_PATTERNS) {
      forbidden.regex.lastIndex = 0;
      let match = forbidden.regex.exec(line);
      while (match) {
        violations.push({
          severity: "DIRECT_DOCKET_ADAPTER_IMPORT",
          file: relativePath,
          line: lineIndex + 1,
          pattern: forbidden.pattern,
          reason: "Kernel/storage consumers must use SandboxAdapter injection, not concrete KERNEL-003 Docker/Docket surfaces.",
        });
        match = forbidden.regex.exec(line);
      }
    }
  }

  return violations;
}

export function scanProductRoot(repoRoot) {
  const resolvedRoot = path.resolve(repoRoot);
  const missing = missingProductRoots(resolvedRoot);
  if (missing.length > 0) {
    return {
      ok: false,
      malformed: true,
      repo_root: normalizePath(resolvedRoot),
      missing_roots: missing,
      checked_files: 0,
      violations: [],
    };
  }

  const files = PRODUCT_ROOTS.flatMap((relativeRoot) => walkRustFiles(path.join(resolvedRoot, relativeRoot)));
  const violations = files.flatMap((filePath) => findViolationsInFile(filePath, resolvedRoot));
  return {
    ok: violations.length === 0,
    malformed: false,
    repo_root: normalizePath(resolvedRoot),
    scanned_roots: PRODUCT_ROOTS,
    checked_files: files.length,
    violations,
  };
}

export function runCli(argv = process.argv.slice(2)) {
  let args;
  try {
    args = parseArgs(argv);
  } catch (error) {
    console.error(`[NO_DIRECT_DOCKET_ADAPTER_IMPORT] ${error.message}`);
    console.error(usage());
    return EXIT_USAGE_OR_MALFORMED;
  }

  if (args.help) {
    console.log(usage());
    return EXIT_OK;
  }

  const productRoot = resolveProductRoot(args.repoRoot);
  const result = scanProductRoot(productRoot);

  if (result.malformed) {
    console.error([
      `[NO_DIRECT_DOCKET_ADAPTER_IMPORT] missing required product roots under ${result.repo_root}:`,
      ...result.missing_roots.map((root) => `- ${root}`),
    ].join("\n"));
    return EXIT_USAGE_OR_MALFORMED;
  }

  if (result.violations.length > 0) {
    for (const violation of result.violations) {
      console.log(JSON.stringify(violation));
    }
    console.error(
      `[NO_DIRECT_DOCKET_ADAPTER_IMPORT] direct KERNEL-003 Docker/Docket references found: ${result.violations.length} violation(s).`,
    );
    return EXIT_VIOLATION;
  }

  console.log(
    `no-direct-docket-adapter-import-check ok (${result.checked_files} file(s) checked in ${result.repo_root})`,
  );
  return EXIT_OK;
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  const invoked = fs.realpathSync.native(path.resolve(process.argv[1]));
  const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
  return invoked === current;
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
