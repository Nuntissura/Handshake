import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const CHECK_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_REPO_ROOT = path.resolve(CHECK_DIR, "../../..");
const SCAN_ROOTS = ["src", "app/src-tauri"];
const CODE_EXTENSIONS = new Set([
  ".rs",
  ".js",
  ".jsx",
  ".ts",
  ".tsx",
  ".mjs",
  ".cjs",
]);
const SKIP_DIRS = new Set([
  ".git",
  ".GOV",
  "node_modules",
  "target",
  "dist",
  "build",
  ".next",
  ".vite",
]);

// HBR-QUIET API ban allowlist: direct foreground/focus APIs may only appear in
// src/**/operator_foreground/** and **/quiet_window.rs. Clippy cannot express
// this path scope, so this checker is the canonical path-scoped fallback.
const ALLOWLIST = {
  operatorForegroundFragment: "/operator_foreground/",
  quietWindowSuffix: "/quiet_window.rs",
};

const FORBIDDEN_APIS = [
  {
    api_name: "set_focus",
    reason: "HBR-QUIET-001",
    pattern: /\.\s*set_focus\b/g,
  },
  {
    api_name: "show",
    reason: "HBR-QUIET-001",
    pattern: /\.\s*show\b/g,
  },
  {
    api_name: "unminimize",
    reason: "HBR-QUIET-001",
    pattern: /\.\s*unminimize\b/g,
  },
  {
    api_name: "AllowSetForegroundWindow",
    reason: "HBR-QUIET-002",
    pattern: /\bAllowSetForegroundWindow\b/g,
  },
  {
    api_name: "AttachThreadInput",
    reason: "HBR-QUIET-002",
    pattern: /\bAttachThreadInput\b/g,
  },
  {
    api_name: "SetForegroundWindow",
    reason: "HBR-QUIET-002",
    pattern: /\bSetForegroundWindow\b/g,
  },
  {
    api_name: "LockSetForegroundWindow",
    reason: "HBR-QUIET-002",
    pattern: /\bLockSetForegroundWindow\b/g,
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
    "Usage: node .GOV/roles_shared/checks/hbr-quiet-api-lint.mjs [--repo-root <path>]",
    "",
    "Scans src/ and app/src-tauri/ for HBR-QUIET-001/002 forbidden foreground APIs.",
    "Violations are emitted as JSONL to stdout and exit with code 2.",
  ].join("\n");
}

function resolveRepoRoot(repoRootArg) {
  if (repoRootArg) return path.resolve(repoRootArg);
  const envRoot = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  if (envRoot) return path.resolve(envRoot);
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

function isAllowlisted(relativePath) {
  const normalized = `/${relativePath.replace(/\\/g, "/")}`;
  return normalized.includes(ALLOWLIST.operatorForegroundFragment)
    || normalized.endsWith(ALLOWLIST.quietWindowSuffix);
}

function shouldScanFile(filePath) {
  return CODE_EXTENSIONS.has(path.extname(filePath).toLowerCase());
}

function walkFiles(rootDir) {
  if (!fs.existsSync(rootDir)) return [];
  const files = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const current = stack.pop();
    const entries = fs.readdirSync(current, { withFileTypes: true });
    for (const entry of entries) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        if (!SKIP_DIRS.has(entry.name)) stack.push(entryPath);
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
  let i = 0;
  let blockDepth = 0;
  let stringQuote = "";
  let rawTerminator = "";

  while (i < text.length) {
    const char = text[i];
    const next = text[i + 1] || "";

    if (char === "\r" || char === "\n") {
      output += char;
      i += 1;
      continue;
    }

    if (blockDepth > 0) {
      if (char === "/" && next === "*") {
        output += "  ";
        blockDepth += 1;
        i += 2;
      } else if (char === "*" && next === "/") {
        output += "  ";
        blockDepth -= 1;
        i += 2;
      } else {
        output += " ";
        i += 1;
      }
      continue;
    }

    if (rawTerminator) {
      if (text.startsWith(rawTerminator, i)) {
        output += " ".repeat(rawTerminator.length);
        i += rawTerminator.length;
        rawTerminator = "";
      } else {
        output += " ";
        i += 1;
      }
      continue;
    }

    if (stringQuote) {
      if (char === "\\" && stringQuote !== "`") {
        output += " ";
        if (i + 1 < text.length && next !== "\r" && next !== "\n") {
          output += " ";
          i += 2;
        } else {
          i += 1;
        }
      } else if (char === stringQuote) {
        output += " ";
        stringQuote = "";
        i += 1;
      } else {
        output += " ";
        i += 1;
      }
      continue;
    }

    if (char === "/" && next === "/") {
      output += "  ";
      i += 2;
      while (i < text.length && text[i] !== "\n" && text[i] !== "\r") {
        output += " ";
        i += 1;
      }
      continue;
    }

    if (char === "/" && next === "*") {
      output += "  ";
      blockDepth = 1;
      i += 2;
      continue;
    }

    const rawStart = rawStringStart(text, i);
    if (rawStart) {
      output += " ".repeat(rawStart.prefixLength);
      i += rawStart.prefixLength;
      rawTerminator = rawStart.terminator;
      continue;
    }

    if (char === '"' || char === "'" || char === "`") {
      output += " ";
      stringQuote = char;
      i += 1;
      continue;
    }

    output += char;
    i += 1;
  }

  return output;
}

function findViolationsInFile(filePath, repoRoot) {
  const relativePath = normalizeRelative(filePath, repoRoot);
  if (isAllowlisted(relativePath)) return [];

  const source = fs.readFileSync(filePath, "utf8");
  const masked = maskSourceTrivia(source);
  const lines = masked.split(/\r?\n/);
  const violations = [];

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    for (const api of FORBIDDEN_APIS) {
      api.pattern.lastIndex = 0;
      let match = api.pattern.exec(line);
      while (match) {
        violations.push({
          file: relativePath,
          line: lineIndex + 1,
          api_name: api.api_name,
          reason: api.reason,
        });
        match = api.pattern.exec(line);
      }
    }
  }

  return violations;
}

function scanRepo(repoRoot) {
  const allFiles = SCAN_ROOTS.flatMap((relativeRoot) => walkFiles(path.join(repoRoot, relativeRoot)));
  const violations = allFiles.flatMap((filePath) => findViolationsInFile(filePath, repoRoot));
  return {
    checked_files: allFiles.length,
    violations,
  };
}

function main() {
  let args;
  try {
    args = parseArgs(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    console.error(usage());
    process.exit(1);
  }

  if (args.help) {
    console.log(usage());
    process.exit(0);
  }

  const repoRoot = resolveRepoRoot(args.repoRoot);
  const result = scanRepo(repoRoot);
  if (result.violations.length > 0) {
    for (const violation of result.violations) {
      console.log(JSON.stringify(violation));
    }
    process.exit(2);
  }

  console.log(`hbr-quiet-api-lint ok (${result.checked_files} file(s) checked)`);
}

main();
