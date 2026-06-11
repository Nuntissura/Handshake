#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/*
Research basis for MT-012 / HBR-MAN-003:
- Tauri v2 documents that callable frontend-to-Rust commands are Rust functions annotated with
  `#[tauri::command]` and registered through `generate_handler!`.
  https://v2.tauri.app/develop/calling-rust/
- ripgrep's documented boundary semantics use `\b` for whole-token matching; this scanner uses the
  same boundary idea in JavaScript regexes so `id` does not satisfy `candidate_id`.
  https://ripgrep.dev/docs/regex/
- Node child_process docs recommend argument arrays/no shell when external processes are needed.
  This checker stays in-process for deterministic Windows behavior while preserving grep-style
  exact and boundary-based source matching.
  https://nodejs.org/api/child_process.html

Selected approach:
- Parse the static Rust ModelManual authority file and extract `CommandReference` blocks.
- Enforce only `CommandStatus::Wired` entries; `CommandStatus::Planned` is the HBR-MAN-003
  not-applicable escape hatch for roadmap surfaces.
- Resolve wired surfaces against product Rust roots plus governance script/schema roots. Current
  ModelManual wired HBR entries are governance code surfaces, while process-ledger entries are
  Rust/Postgres product surfaces.
*/

const MANUAL_CONTENT_PATH = "src/backend/handshake_core/src/model_manual/content.rs";
const SOURCE_EXTENSIONS = new Set([".rs", ".mjs", ".js", ".json", ".toml", ".yml", ".yaml", ".md"]);
const SKIP_DIRS = new Set([".git", "node_modules", "target", "dist", "build", "tests", "__tests__"]);

class CliError extends Error {
  constructor(message) {
    super(message);
    this.name = "CliError";
  }
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function parseArgs(args) {
  const currentDir = path.dirname(fileURLToPath(import.meta.url));
  const options = {
    repoRoot: String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim() || process.cwd(),
    govRoot: String(process.env.HANDSHAKE_GOV_ROOT || "").trim() || path.resolve(currentDir, "../.."),
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--repo-root") {
      const value = args[index + 1];
      if (!isNonEmptyString(value)) throw new CliError("--repo-root requires a path");
      options.repoRoot = value.trim();
      index += 1;
      continue;
    }
    if (arg === "--gov-root") {
      const value = args[index + 1];
      if (!isNonEmptyString(value)) throw new CliError("--gov-root requires a path");
      options.govRoot = value.trim();
      index += 1;
      continue;
    }
    throw new CliError(`unknown argument: ${arg}`);
  }

  options.repoRoot = path.resolve(options.repoRoot);
  options.govRoot = path.resolve(options.govRoot);
  return options;
}

function toPosix(value) {
  return String(value || "").replace(/\\/g, "/");
}

function readText(absPath) {
  return fs.readFileSync(absPath, "utf8");
}

function readTextIfExists(absPath) {
  return fs.existsSync(absPath) ? readText(absPath) : "";
}

function escapeRegex(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isWordToken(value) {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(value);
}

function wordRegex(value, suffix = "") {
  const escaped = escapeRegex(value);
  return isWordToken(value)
    ? new RegExp(`\\b${escaped}\\b${suffix}`, "m")
    : new RegExp(escaped, "m");
}

function stringLiteralAppears(text, value) {
  return text.includes(`"${value}"`) || text.includes(`'${value}'`);
}

function findCommandReferenceBlocks(content) {
  const blocks = [];
  const marker = "CommandReference";
  let searchIndex = 0;
  while (searchIndex < content.length) {
    const markerIndex = content.indexOf(marker, searchIndex);
    if (markerIndex === -1) break;
    const openIndex = content.indexOf("{", markerIndex);
    if (openIndex === -1) break;
    let depth = 0;
    let inString = false;
    let escaped = false;
    for (let index = openIndex; index < content.length; index += 1) {
      const char = content[index];
      if (inString) {
        if (escaped) {
          escaped = false;
        } else if (char === "\\") {
          escaped = true;
        } else if (char === "\"") {
          inString = false;
        }
        continue;
      }
      if (char === "\"") {
        inString = true;
        continue;
      }
      if (char === "{") depth += 1;
      if (char === "}") {
        depth -= 1;
        if (depth === 0) {
          blocks.push(content.slice(openIndex + 1, index));
          searchIndex = index + 1;
          break;
        }
      }
    }
    if (searchIndex <= markerIndex) break;
  }
  return blocks;
}

function stringField(block, fieldName) {
  const match = block.match(new RegExp(`\\b${escapeRegex(fieldName)}\\s*:\\s*"([^"]*)"`));
  return match ? match[1] : null;
}

function optionStringField(block, fieldName) {
  const match = block.match(new RegExp(`\\b${escapeRegex(fieldName)}\\s*:\\s*Some\\("([^"]*)"\\)`));
  return match ? match[1] : null;
}

function statusField(block) {
  const match = block.match(/\bstatus\s*:\s*CommandStatus::([A-Za-z_][A-Za-z0-9_]*)/);
  return match ? match[1] : null;
}

function stringArrayField(block, fieldName) {
  const match = block.match(new RegExp(`\\b${escapeRegex(fieldName)}\\s*:\\s*&\\[([\\s\\S]*?)\\]`));
  if (!match) return [];
  return [...match[1].matchAll(/"([^"]*)"/g)].map((entry) => entry[1]);
}

function parseManualEntries(content) {
  return findCommandReferenceBlocks(content).map((block, index) => ({
    id: stringField(block, "id") || `<command-${index + 1}>`,
    name: stringField(block, "name") || "",
    status: statusField(block) || "",
    ipc_channel: optionStringField(block, "ipc_channel"),
    tauri_command: optionStringField(block, "tauri_command"),
    schema_fields: stringArrayField(block, "schema_fields"),
    cli_flag: optionStringField(block, "cli_flag"),
  }));
}

function shouldSkipPath(absPath) {
  const parts = toPosix(absPath).split("/");
  if (parts.some((part) => SKIP_DIRS.has(part))) return true;
  return /\/model_manual\//.test(toPosix(absPath));
}

function shouldIndexFile(absPath) {
  if (shouldSkipPath(absPath)) return false;
  if (path.basename(absPath) === "justfile") return true;
  return SOURCE_EXTENSIONS.has(path.extname(absPath));
}

function collectFiles(startAbs) {
  if (!fs.existsSync(startAbs)) return [];
  const stat = fs.statSync(startAbs);
  if (stat.isFile()) return shouldIndexFile(startAbs) ? [startAbs] : [];
  if (!stat.isDirectory()) return [];
  const out = [];
  const visit = (dirAbs) => {
    if (shouldSkipPath(dirAbs)) return;
    for (const dirent of fs.readdirSync(dirAbs, { withFileTypes: true })) {
      const childAbs = path.join(dirAbs, dirent.name);
      if (dirent.isDirectory()) {
        visit(childAbs);
      } else if (dirent.isFile() && shouldIndexFile(childAbs)) {
        out.push(childAbs);
      }
    }
  };
  visit(startAbs);
  return out;
}

function sourceRoots(repoRoot, govRoot) {
  return [
    path.join(repoRoot, "src/backend/handshake_core/src"),
    path.join(repoRoot, "app/src-tauri/src"),
    path.join(govRoot, "roles_shared/checks"),
    path.join(govRoot, "roles_shared/scripts"),
    path.join(govRoot, "roles_shared/schemas"),
    path.join(govRoot, "roles/validator/checks"),
    path.join(govRoot, "roles/validator/scripts"),
    path.join(govRoot, "..", "justfile"),
  ];
}

function buildSourceIndex(repoRoot, govRoot) {
  const files = [...new Set(sourceRoots(repoRoot, govRoot).flatMap(collectFiles))]
    .sort((left, right) => left.localeCompare(right));
  return files.map((file) => ({
    file,
    posix: toPosix(file),
    ext: path.extname(file),
    basename: path.basename(file),
    text: readTextIfExists(file),
  }));
}

function commandNameAliases(name) {
  const aliases = new Set([name]);
  aliases.add(name.replaceAll("_", "-"));
  aliases.add(name.replaceAll("-", "_"));
  return [...aliases].filter(Boolean);
}

function hasCommandName(index, name) {
  const aliases = commandNameAliases(name);
  return index.some((source) =>
    aliases.some((alias) =>
      source.posix.includes(alias)
      || stringLiteralAppears(source.text, alias)
      || wordRegex(alias).test(source.text)
    )
  );
}

function hasIpcChannel(index, name) {
  return index.some((source) => stringLiteralAppears(source.text, name));
}

function hasTauriCommand(index, name) {
  const fn = escapeRegex(name);
  const attrThenFn = new RegExp(
    `#\\s*\\[\\s*tauri::command[^\\]]*\\][\\s\\S]{0,600}?\\b(?:pub(?:\\([^)]*\\))?\\s+)?(?:async\\s+)?fn\\s+${fn}\\s*\\(`,
    "m",
  );
  return index.some((source) => source.ext === ".rs" && attrThenFn.test(source.text));
}

function hasSchemaField(index, name) {
  const rustField = new RegExp(`\\b(?:pub\\s+)?${escapeRegex(name)}\\s*:`, "m");
  return index.some((source) =>
    rustField.test(source.text)
    || stringLiteralAppears(source.text, name)
  );
}

function hasCliFlag(index, name) {
  return index.some((source) => source.text.includes(name));
}

function obligationsForEntry(entry) {
  const out = [];
  if (entry.status !== "Wired") return out;
  if (entry.tauri_command || entry.ipc_channel || entry.cli_flag) {
    out.push({ manual_id: entry.id, kind: "command_name", name: entry.name || entry.id });
  }
  if (entry.ipc_channel) out.push({ manual_id: entry.id, kind: "ipc_channel", name: entry.ipc_channel });
  if (entry.tauri_command) out.push({ manual_id: entry.id, kind: "tauri_command", name: entry.tauri_command });
  for (const field of entry.schema_fields || []) {
    out.push({ manual_id: entry.id, kind: "schema_field", name: field });
  }
  if (entry.cli_flag) out.push({ manual_id: entry.id, kind: "cli_flag", name: entry.cli_flag });
  return out;
}

function resolves(obligation, index) {
  if (!obligation.name || obligation.name === "<unknown>") return false;
  if (obligation.kind === "command_name") return hasCommandName(index, obligation.name);
  if (obligation.kind === "ipc_channel") return hasIpcChannel(index, obligation.name);
  if (obligation.kind === "tauri_command") return hasTauriCommand(index, obligation.name);
  if (obligation.kind === "schema_field") return hasSchemaField(index, obligation.name);
  if (obligation.kind === "cli_flag") return hasCliFlag(index, obligation.name);
  return false;
}

function scanManual({ repoRoot, govRoot }) {
  const manualPath = path.join(repoRoot, MANUAL_CONTENT_PATH);
  if (!fs.existsSync(manualPath)) {
    throw new CliError(`ModelManual content missing: ${manualPath}`);
  }
  const manual = parseManualEntries(readText(manualPath));
  const index = buildSourceIndex(repoRoot, govRoot);
  const obligations = manual.flatMap(obligationsForEntry);
  const failures = obligations
    .filter((obligation) => !resolves(obligation, index))
    .map((obligation) => ({
      manual_id: obligation.manual_id,
      name: obligation.name,
      kind: obligation.kind,
      reason: "no_source_match",
    }));
  return {
    manual_entries: manual.length,
    checked_obligations: obligations.length,
    failures,
  };
}

function emitJsonLines(records) {
  for (const record of records) {
    console.error(JSON.stringify(record));
  }
}

function errorRecord(error) {
  return {
    manual_id: null,
    name: null,
    kind: null,
    reason: error?.message || String(error || "unknown error"),
  };
}

export function runCli(args = process.argv.slice(2)) {
  try {
    const options = parseArgs(args);
    const result = scanManual(options);
    if (result.failures.length > 0) {
      emitJsonLines(result.failures);
      return 2;
    }
    console.log(`hbr-man-003-scan ok (${result.checked_obligations} wired manual name checks)`);
    return 0;
  } catch (error) {
    emitJsonLines([errorRecord(error)]);
    return error instanceof CliError ? 3 : 1;
  }
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  return fs.realpathSync.native(path.resolve(process.argv[1]))
    === fs.realpathSync.native(fileURLToPath(import.meta.url));
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
