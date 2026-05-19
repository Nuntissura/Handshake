#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

const FOREGROUND_EXEMPT_COMMANDS = [];

const FORBIDDEN_PATTERNS = [
  {
    code: "FORBIDDEN_OS_FOREGROUND_API",
    label: "SetForegroundWindow",
    pattern: /\bSetForegroundWindow\b/g,
  },
  {
    code: "FORBIDDEN_OS_FOREGROUND_API",
    label: "AllowSetForegroundWindow",
    pattern: /\bAllowSetForegroundWindow\b/g,
  },
  {
    code: "FORBIDDEN_OS_FOREGROUND_API",
    label: "LockSetForegroundWindow",
    pattern: /\bLockSetForegroundWindow\b/g,
  },
  {
    code: "FORBIDDEN_OS_INPUT_API",
    label: "SendInput",
    pattern: /\bSendInput\b/g,
  },
  {
    code: "FORBIDDEN_TAURI_FOCUS_API",
    label: ".set_focus(",
    pattern: /\.set_focus\s*\(/g,
  },
  {
    code: "FORBIDDEN_TAURI_FOCUS_API",
    label: ".unminimize(",
    pattern: /\.unminimize\s*\(/g,
  },
  {
    code: "FORBIDDEN_TAURI_WINDOW_SHOW_API",
    label: "WebviewWindowBuilder::new",
    pattern: /\bWebviewWindowBuilder\s*::\s*new\b/g,
  },
];

function parseArgs(argv) {
  const args = {
    repoRoot: process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "",
    json: false,
    report: false,
    out: "",
    fixture: "",
  };

  for (let index = 2; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--repo-root") {
      args.repoRoot = argv[++index] || "";
    } else if (arg === "--json") {
      args.json = true;
    } else if (arg === "--report") {
      args.report = true;
    } else if (arg === "--out") {
      args.out = argv[++index] || "";
    } else if (arg === "--fixture") {
      args.fixture = argv[++index] || "";
    } else if (arg === "--help" || arg === "-h") {
      printUsage();
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return args;
}

function printUsage() {
  console.log(`Usage: node .GOV/roles_shared/scripts/automation-first-audit.mjs [--repo-root <path>] [--json] [--report --out <path>] [--fixture synthetic-violation]`);
}

function resolveRepoRoot(repoRootArg) {
  if (repoRootArg && repoRootArg.trim()) {
    return path.resolve(repoRootArg);
  }
  try {
    return execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return process.cwd();
  }
}

function walkRsFiles(root) {
  const out = [];
  function visit(dir) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        visit(full);
      } else if (entry.isFile() && entry.name.endsWith(".rs")) {
        out.push(full);
      }
    }
  }
  visit(root);
  return out.sort();
}

function stripLineComments(source) {
  return source.replace(/\/\/.*$/gm, "");
}

function discoverCommands(appSrcRoot) {
  const files = walkRsFiles(appSrcRoot);
  const commands = [];
  const commandRe = /#\s*\[\s*tauri::command(?:\([^)]*\))?\s*\]\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/gms;

  for (const file of files) {
    const source = readFileSync(file, "utf8");
    const moduleName = path.basename(file, ".rs") === "lib" ? "" : path.basename(file, ".rs");
    for (const match of source.matchAll(commandRe)) {
      const name = match[1];
      commands.push({
        name,
        module: moduleName,
        handler_ref: moduleName ? `${moduleName}::${name}` : name,
        file: normalizeSlash(path.relative(appSrcRoot, file)),
        source,
      });
    }
  }
  return commands.sort((a, b) => a.handler_ref.localeCompare(b.handler_ref));
}

function discoverRegisteredHandlers(libSource) {
  const marker = "tauri::generate_handler![";
  const entries = new Set();

  // Walk every occurrence of the marker. The lib.rs declares both arms of a
  // macro_rules! `handshake_invoke_handlers!` wrapper, each containing a
  // `tauri::generate_handler![...]` block, and other repos may grow further
  // blocks; we want the union.
  let scanFrom = 0;
  while (scanFrom < libSource.length) {
    const start = libSource.indexOf(marker, scanFrom);
    if (start < 0) break;
    const contentStart = start + marker.length;
    let depth = 1;
    let index = contentStart;
    while (index < libSource.length && depth > 0) {
      const char = libSource[index];
      if (char === "[") depth += 1;
      if (char === "]") depth -= 1;
      index += 1;
    }
    const block = stripLineComments(libSource.slice(contentStart, index - 1));
    for (const raw of block.split(",")) {
      const entry = raw.trim();
      if (!entry) continue;
      entries.add(entry);
      // Also register the trailing `module::name` and bare `name` so the
      // command-inventory matcher (which only knows the file basename as the
      // module) can resolve commands nested under additional namespaces such
      // as `commands::steering::...` -> `steering::...` -> `...`.
      const segments = entry.split("::");
      if (segments.length >= 2) {
        entries.add(segments.slice(-2).join("::"));
      }
      entries.add(segments[segments.length - 1]);
    }
    scanFrom = index;
  }

  // Recognize commands referenced as `$extra:path` arguments to
  // `handshake_invoke_handlers!(name1, name2, ...)`. Those expand into a
  // `generate_handler!` block at compile time but only the macro invocation
  // is in source. We treat each path token as a registered handler.
  const extraInvocationRe = /handshake_invoke_handlers!\s*\(\s*([^)]*)\)/g;
  for (const match of libSource.matchAll(extraInvocationRe)) {
    const argList = stripLineComments(match[1] || "");
    for (const raw of argList.split(",")) {
      const entry = raw.trim();
      if (!entry || entry.startsWith("$")) continue;
      entries.add(entry);
      const segments = entry.split("::");
      if (segments.length >= 2) {
        entries.add(segments.slice(-2).join("::"));
      }
      entries.add(segments[segments.length - 1]);
    }
  }

  return Array.from(entries).sort();
}

function scanForbidden(command) {
  const source = command.source;
  const hits = [];
  for (const rule of FORBIDDEN_PATTERNS) {
    for (const match of source.matchAll(rule.pattern)) {
      hits.push({
        code: rule.code,
        api: rule.label,
        offset: match.index || 0,
      });
    }
  }
  return hits;
}

function auditRepo(repoRoot) {
  const appSrcRoot = path.join(repoRoot, "app", "src-tauri", "src");
  const libPath = path.join(appSrcRoot, "lib.rs");
  if (!existsSync(appSrcRoot)) {
    throw new Error(`Tauri source root not found: ${appSrcRoot}`);
  }
  if (!existsSync(libPath)) {
    throw new Error(`Tauri lib.rs not found: ${libPath}`);
  }

  const commands = discoverCommands(appSrcRoot);
  const registered = discoverRegisteredHandlers(readFileSync(libPath, "utf8"));
  return buildAudit({
    mode: "repo",
    commands,
    registered,
  });
}

function auditSyntheticViolation() {
  const source = `
#[tauri::command]
fn synthetic_bad_command() {
    unsafe {
        SetForegroundWindow(hwnd);
    }
}
`;
  return buildAudit({
    mode: "synthetic-violation",
    commands: [{
      name: "synthetic_bad_command",
      module: "",
      handler_ref: "synthetic_bad_command",
      file: "synthetic.rs",
      source,
    }],
    registered: ["synthetic_bad_command"],
  });
}

function buildAudit({ mode, commands, registered }) {
  const registeredSet = new Set(registered);
  const exemptSet = new Set(FOREGROUND_EXEMPT_COMMANDS);
  const commandReports = commands.map((command) => {
    const registeredByExactRef = registeredSet.has(command.handler_ref);
    const registeredByBaseName = registeredSet.has(command.name);
    const ipcCallable = registeredByExactRef || registeredByBaseName;
    const forbidden_hits = scanForbidden(command);
    const foregroundExempt = exemptSet.has(command.handler_ref) || exemptSet.has(command.name);
    const violations = [];

    if (!ipcCallable) {
      violations.push({
        code: "COMMAND_NOT_REGISTERED_FOR_IPC",
        message: `${command.handler_ref} has #[tauri::command] but is missing from generate_handler!`,
      });
    }
    for (const hit of forbidden_hits) {
      violations.push({
        code: hit.code,
        message: `${command.handler_ref} references forbidden automation API ${hit.api}`,
      });
    }
    if (foregroundExempt) {
      violations.push({
        code: "FOREGROUND_EXEMPTION_NOT_ALLOWED",
        message: `${command.handler_ref} is in foreground_exempt_commands; MT-020 allowlist must remain empty`,
      });
    }

    return {
      command: command.handler_ref,
      file: command.file,
      ipc_callable: ipcCallable,
      foreground_exempt: foregroundExempt,
      keyboard_injection_invocation_count: 0,
      focus_steal_api_count: forbidden_hits.length,
      violations,
      status: violations.length === 0 ? "PASS" : "FAIL",
    };
  });

  const violations = commandReports.flatMap((command) =>
    command.violations.map((violation) => ({
      command: command.command,
      file: command.file,
      ...violation,
    })),
  );

  return {
    schema_id: "hsk.automation_first_audit@1",
    mode,
    generated_at_utc: new Date().toISOString(),
    foreground_exempt_commands: [...FOREGROUND_EXEMPT_COMMANDS],
    command_count: commandReports.length,
    registered_handler_count: registered.length,
    commands: commandReports,
    violations,
    status: violations.length === 0 ? "PASS" : "FAIL",
  };
}

function renderMarkdown(audit) {
  const lines = [
    "# Automation-First Audit",
    "",
    `- status: ${audit.status}`,
    `- command_count: ${audit.command_count}`,
    `- foreground_exempt_commands: ${audit.foreground_exempt_commands.length}`,
    "",
    "## Commands",
    "",
    "| command | status | ipc | focus/input hits |",
    "| --- | --- | --- | --- |",
  ];

  for (const command of audit.commands) {
    lines.push(`| ${command.command} | ${command.status} | ${command.ipc_callable ? "yes" : "no"} | ${command.focus_steal_api_count} |`);
  }

  if (audit.violations.length > 0) {
    lines.push("", "## Violations", "");
    for (const violation of audit.violations) {
      lines.push(`- ${violation.code}: ${violation.message}`);
    }
  }

  lines.push("");
  return lines.join("\n");
}

function writeReport(outPath, audit) {
  const resolved = path.resolve(outPath.replace(/^\/tmp\//, `${os.tmpdir()}${path.sep}`));
  mkdirSync(path.dirname(resolved), { recursive: true });
  writeFileSync(resolved, renderMarkdown(audit), "utf8");
  return resolved;
}

function normalizeSlash(value) {
  return value.split(path.sep).join("/");
}

function main() {
  const args = parseArgs(process.argv);
  const audit = args.fixture === "synthetic-violation"
    ? auditSyntheticViolation()
    : auditRepo(resolveRepoRoot(args.repoRoot));

  if (args.report) {
    if (!args.out) throw new Error("--report requires --out <path>");
    const out = writeReport(args.out, audit);
    console.log(`automation-first-audit report: ${out}`);
  }
  if (args.json) {
    console.log(JSON.stringify(audit, null, 2));
  }
  if (!args.json && !args.report) {
    console.log(`automation-first-audit ${audit.status} (${audit.command_count} command(s), ${audit.violations.length} violation(s))`);
  }
  if (audit.status !== "PASS") {
    process.exitCode = 1;
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
