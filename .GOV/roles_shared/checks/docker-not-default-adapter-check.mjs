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

const BOOTSTRAP_RELATIVE_PATH = "src/backend/handshake_core/src/sandbox/bootstrap.rs";
const SETTINGS_RELATIVE_ROOT = "settings";

function parseArgs(argv) {
  const args = {
    repoRoot: "",
    govRoot: "",
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
    if (arg === "--gov-root") {
      args.govRoot = String(argv[index + 1] || "").trim();
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
    "Usage: node .GOV/roles_shared/checks/docker-not-default-adapter-check.mjs [--repo-root <product-root>] [--gov-root <gov-root>]",
    "",
    "Scans sandbox bootstrap and .GOV/settings JSON defaults to ensure Docker remains compat-only and never becomes the default adapter.",
    "Violations are emitted as JSONL to stderr and exit with code 2.",
  ].join("\n");
}

function normalizePath(filePath) {
  return filePath.replace(/\\/g, "/");
}

function normalizeRelative(filePath, root) {
  return normalizePath(path.relative(root, filePath));
}

function productRootLooksValid(repoRoot) {
  const bootstrapPath = path.join(repoRoot, BOOTSTRAP_RELATIVE_PATH);
  return fs.existsSync(bootstrapPath) && fs.statSync(bootstrapPath).isFile();
}

function resolveProductRoot(repoRootArg) {
  if (repoRootArg) return path.resolve(repoRootArg);

  const envProductRoot = String(process.env.HANDSHAKE_PRODUCT_REPO_ROOT || "").trim();
  if (envProductRoot) return path.resolve(envProductRoot);

  const activeRepoRoot = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  if (activeRepoRoot && productRootLooksValid(path.resolve(activeRepoRoot))) {
    return path.resolve(activeRepoRoot);
  }

  if (productRootLooksValid(GOV_WORKTREE_ROOT)) return GOV_WORKTREE_ROOT;

  return DEFAULT_PRODUCT_WORKTREE_ROOT;
}

function resolveGovRoot(govRootArg, repoRoot) {
  if (govRootArg) return path.resolve(govRootArg);

  const envGovRoot = String(process.env.HANDSHAKE_GOV_ROOT || "").trim();
  if (envGovRoot) return path.resolve(envGovRoot);

  const repoLocalGov = path.join(repoRoot, ".GOV");
  if (fs.existsSync(repoLocalGov)) return repoLocalGov;

  return path.resolve(GOV_WORKTREE_ROOT, ".GOV");
}

function lineNumberAt(text, index) {
  return text.slice(0, index).split(/\r?\n/).length;
}

function stripCommentsPreserveStrings(text) {
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
      output += char;
      if (text.startsWith(rawTerminator, index)) {
        output += rawTerminator.slice(1);
        index += rawTerminator.length;
        rawTerminator = "";
      } else {
        index += 1;
      }
      continue;
    }

    if (stringQuote) {
      output += char;
      if (char === "\\" && stringQuote !== "`") {
        if (index + 1 < text.length) {
          output += text[index + 1];
          index += 2;
        } else {
          index += 1;
        }
      } else if (char === stringQuote) {
        stringQuote = "";
        index += 1;
      } else {
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
      output += text.slice(index, index + rawStart.prefixLength);
      index += rawStart.prefixLength;
      rawTerminator = rawStart.terminator;
      continue;
    }

    if (char === '"' || char === "'" || char === "`") {
      output += char;
      stringQuote = char;
      index += 1;
      continue;
    }

    output += char;
    index += 1;
  }

  return output;
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

function bootstrapViolations(repoRoot) {
  const bootstrapPath = path.join(repoRoot, BOOTSTRAP_RELATIVE_PATH);
  if (!fs.existsSync(bootstrapPath)) {
    return {
      malformed: {
        message: `missing required sandbox bootstrap file: ${BOOTSTRAP_RELATIVE_PATH}`,
      },
      violations: [],
    };
  }

  const source = fs.readFileSync(bootstrapPath, "utf8");
  const uncommented = stripCommentsPreserveStrings(source);
  const patterns = [
    {
      regex: /SandboxAdapterRegistry::new\s*\(\s*(?:AdapterId::new\s*\(\s*)?DOCKER_ADAPTER_ID/g,
      evidence: "SandboxAdapterRegistry::new(DOCKER_ADAPTER_ID)",
    },
    {
      regex: /SandboxAdapterRegistry::new\s*\(\s*(?:AdapterId::new\s*\(\s*)?"docker"/g,
      evidence: "SandboxAdapterRegistry::new(\"docker\")",
    },
    {
      regex: /(?:default_adapter_id|preferred_default_adapter_id)\s*=\s*(?:AdapterId::new\s*\(\s*)?DOCKER_ADAPTER_ID/g,
      evidence: "default_adapter_id = DOCKER_ADAPTER_ID",
    },
    {
      regex: /(?:default_adapter_id|preferred_default_adapter_id)\s*=\s*(?:AdapterId::new\s*\(\s*)?"docker"/g,
      evidence: "default_adapter_id = \"docker\"",
    },
  ];

  const violations = [];
  for (const pattern of patterns) {
    let match = pattern.regex.exec(uncommented);
    while (match) {
      violations.push({
        severity: "DOCKER_DEFAULT_ADAPTER",
        file: BOOTSTRAP_RELATIVE_PATH,
        line: lineNumberAt(uncommented, match.index),
        evidence: pattern.evidence,
        reason: "Docker must remain compat-only and must not be passed as a registry default.",
      });
      match = pattern.regex.exec(uncommented);
    }
  }
  return { malformed: null, violations };
}

function walkJsonFiles(rootDir) {
  if (!fs.existsSync(rootDir)) return [];
  const files = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(entryPath);
      } else if (entry.isFile() && entry.name.toLowerCase().endsWith(".json")) {
        files.push(entryPath);
      }
    }
  }
  return files.sort((a, b) => a.localeCompare(b));
}

function findDockerDefaults(value, pathParts = []) {
  const findings = [];
  if (Array.isArray(value)) {
    value.forEach((entry, index) => {
      findings.push(...findDockerDefaults(entry, [...pathParts, String(index)]));
    });
    return findings;
  }
  if (!value || typeof value !== "object") return findings;

  for (const [key, entry] of Object.entries(value)) {
    const nextPath = [...pathParts, key];
    const normalizedKey = key.toLowerCase();
    if (
      (normalizedKey === "default_adapter" || normalizedKey === "default_adapter_id")
      && typeof entry === "string"
      && entry.trim().toLowerCase() === "docker"
    ) {
      findings.push(nextPath.join("."));
    }
    findings.push(...findDockerDefaults(entry, nextPath));
  }
  return findings;
}

function settingsViolations(govRoot) {
  const settingsRoot = path.join(govRoot, SETTINGS_RELATIVE_ROOT);
  const files = walkJsonFiles(settingsRoot);
  const violations = [];
  const malformed = [];

  for (const file of files) {
    let parsed;
    try {
      parsed = JSON.parse(fs.readFileSync(file, "utf8"));
    } catch (error) {
      malformed.push({
        file: normalizeRelative(file, govRoot),
        message: `malformed JSON: ${error.message}`,
      });
      continue;
    }

    for (const jsonPath of findDockerDefaults(parsed)) {
      violations.push({
        severity: "DOCKER_DEFAULT_ADAPTER",
        file: `.GOV/${normalizeRelative(file, govRoot)}`,
        line: 1,
        json_path: jsonPath,
        reason: "Operator settings must not set sandbox.default_adapter to docker; Docker is explicit opt-in compat only.",
      });
    }
  }

  return { malformed, violations, checkedSettingsFiles: files.length };
}

export function scanDockerDefault({ repoRoot, govRoot }) {
  const resolvedRepoRoot = path.resolve(repoRoot);
  const resolvedGovRoot = path.resolve(govRoot);
  const bootstrap = bootstrapViolations(resolvedRepoRoot);
  const settings = settingsViolations(resolvedGovRoot);
  const malformed = [
    ...(bootstrap.malformed ? [bootstrap.malformed] : []),
    ...settings.malformed,
  ];
  const violations = [
    ...bootstrap.violations,
    ...settings.violations,
  ];

  return {
    ok: malformed.length === 0 && violations.length === 0,
    malformed,
    repo_root: normalizePath(resolvedRepoRoot),
    gov_root: normalizePath(resolvedGovRoot),
    checked_files: 1 + settings.checkedSettingsFiles,
    violations,
  };
}

export function runCli(argv = process.argv.slice(2)) {
  let args;
  try {
    args = parseArgs(argv);
  } catch (error) {
    console.error(`[DOCKER_NOT_DEFAULT_ADAPTER] ${error.message}`);
    console.error(usage());
    return EXIT_USAGE_OR_MALFORMED;
  }

  if (args.help) {
    console.log(usage());
    return EXIT_OK;
  }

  const repoRoot = resolveProductRoot(args.repoRoot);
  const govRoot = resolveGovRoot(args.govRoot, repoRoot);
  const result = scanDockerDefault({ repoRoot, govRoot });

  if (result.malformed.length > 0) {
    for (const row of result.malformed) {
      console.error(JSON.stringify({
        severity: "DOCKER_DEFAULT_ADAPTER_CHECK_MALFORMED",
        ...row,
      }));
    }
    return EXIT_USAGE_OR_MALFORMED;
  }

  if (result.violations.length > 0) {
    for (const violation of result.violations) {
      console.error(JSON.stringify(violation));
    }
    console.error(
      `[DOCKER_NOT_DEFAULT_ADAPTER] Docker must remain compat-only: ${result.violations.length} violation(s).`,
    );
    return EXIT_VIOLATION;
  }

  console.log(
    `docker-not-default-adapter-check ok (${result.checked_files} file(s) checked in ${result.repo_root})`,
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
