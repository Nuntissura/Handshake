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
    runtimeProbeEvidence: "",
    requireRuntimeProbe: false,
    staticSourceScanOk: false,
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
    } else if (arg === "--runtime-probe-evidence") {
      args.runtimeProbeEvidence = argv[++index] || "";
    } else if (arg === "--require-runtime-probe") {
      args.requireRuntimeProbe = true;
    } else if (arg === "--static-source-scan-ok") {
      args.staticSourceScanOk = true;
    } else if (arg === "--help" || arg === "-h") {
      printUsage();
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  if (args.requireRuntimeProbe && args.staticSourceScanOk) {
    throw new Error("--require-runtime-probe and --static-source-scan-ok are mutually exclusive");
  }
  args.requireRuntimeProbe = args.requireRuntimeProbe || !args.staticSourceScanOk;

  return args;
}

function printUsage() {
  console.log(`Usage: node .GOV/roles_shared/scripts/automation-first-audit.mjs [--repo-root <path>] [--json] [--report --out <path>] [--fixture synthetic-violation] [--runtime-probe-evidence <path>] [--require-runtime-probe] [--static-source-scan-ok]`);
  console.log("Runtime-probe evidence is required by default. Use --static-source-scan-ok only for explicit non-certifying scanner/report runs.");
}

// MT-020 runtime-probe evidence ingestion.
//
// Without this file the audit's `keyboard_injection_invocation_count` and the
// runtime focus measurement default to STATIC values (regex-derived). That is
// exactly the gap the MT-020 Integration Validator flagged: the report claimed
// zero keyboard-injection / focus-steal events but never *measured* them at
// runtime. When the Rust runtime-probe harness
// (tests/automation_first_audit_runtime_tests.rs) runs the three live probes
// (IPC mock call, IPC under live focus-audit, raw OS SendInput injection), it
// writes a measured-evidence JSON which this function ingests so the report's
// QUIET counters are backed by real observations instead of a hardcoded 0.
//
// Evidence schema (hsk.automation_first_runtime_probe_evidence@1):
//   {
//     "schema_id": "hsk.automation_first_runtime_probe_evidence@1",
//     "platform": "windows" | "non-windows",
//     "focus_audit_measured": true|false,
//     "keyboard_injection_measured": true|false,
//     "commands": {
//       "<handler_ref>": {
//         "ipc_mock_call_ok": true,
//         "focus_steal_event_count": 0,          // measured handshake-owned foreground events
//         "keyboard_injection_invocation_count": 0 // measured command fires under raw injection
//       }, ...
//     }
//   }
function loadRuntimeProbeEvidence(evidencePath) {
  if (!evidencePath || !evidencePath.trim()) return null;
  const resolved = path.resolve(evidencePath.replace(/^\/tmp\//, `${os.tmpdir()}${path.sep}`));
  if (!existsSync(resolved)) {
    throw new Error(`runtime-probe-evidence file not found: ${resolved}`);
  }
  const parsed = JSON.parse(readFileSync(resolved, "utf8"));
  if (parsed.schema_id !== "hsk.automation_first_runtime_probe_evidence@1") {
    throw new Error(
      `runtime-probe-evidence has wrong schema_id: ${parsed.schema_id}`,
    );
  }
  if (!parsed.commands || typeof parsed.commands !== "object") {
    throw new Error("runtime-probe-evidence missing commands map");
  }
  if (typeof parsed.focus_audit_measured !== "boolean") {
    throw new Error("runtime-probe-evidence focus_audit_measured must be boolean");
  }
  if (typeof parsed.keyboard_injection_measured !== "boolean") {
    throw new Error("runtime-probe-evidence keyboard_injection_measured must be boolean");
  }
  for (const [handlerRef, commandEvidence] of Object.entries(parsed.commands)) {
    validateRuntimeCommandEvidence(handlerRef, commandEvidence);
  }
  return parsed;
}

function validateRuntimeCommandEvidence(handlerRef, commandEvidence) {
  if (!commandEvidence || typeof commandEvidence !== "object" || Array.isArray(commandEvidence)) {
    throw new Error(`runtime-probe-evidence command ${handlerRef} must be an object`);
  }
  if (typeof commandEvidence.ipc_mock_call_ok !== "boolean") {
    throw new Error(
      `runtime-probe-evidence command ${handlerRef} missing boolean ipc_mock_call_ok`,
    );
  }
  if (commandEvidence.ipc_mock_call_ok !== true) {
    throw new Error(
      `runtime-probe-evidence command ${handlerRef} ipc_mock_call_ok must be true`,
    );
  }
  for (const field of [
    "focus_steal_event_count",
    "keyboard_injection_invocation_count",
  ]) {
    const value = commandEvidence[field];
    if (!Number.isInteger(value) || value < 0) {
      throw new Error(
        `runtime-probe-evidence command ${handlerRef} missing non-negative integer ${field}`,
      );
    }
  }
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

function auditRepo(repoRoot, runtimeProbeEvidence, requireRuntimeProbe = false) {
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
    runtimeProbeEvidence,
    requireRuntimeProbe,
  });
}

function auditSyntheticViolation(runtimeProbeEvidence = null, requireRuntimeProbe = false) {
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
    runtimeProbeEvidence,
    requireRuntimeProbe,
  });
}

function buildAudit({
  mode,
  commands,
  registered,
  runtimeProbeEvidence = null,
  requireRuntimeProbe = false,
}) {
  const registeredSet = new Set(registered);
  const exemptSet = new Set(FOREGROUND_EXEMPT_COMMANDS);
  const evidenceCommands = runtimeProbeEvidence ? runtimeProbeEvidence.commands : null;
  const commandReports = commands.map((command) => {
    const registeredByExactRef = registeredSet.has(command.handler_ref);
    const registeredByBaseName = registeredSet.has(command.name);
    const ipcCallable = registeredByExactRef || registeredByBaseName;
    const forbidden_hits = scanForbidden(command);
    const foregroundExempt = exemptSet.has(command.handler_ref) || exemptSet.has(command.name);
    const violations = [];

    // Runtime-probe measurement (MT-020). When the Rust harness supplied an
    // evidence file, the keyboard-injection invocation count and focus-steal
    // count are REAL measured values from the three live probes, not the
    // static regex-derived defaults. `evidence_source` records which path was
    // used so the report cannot silently masquerade static zeros as measured.
    const measured = evidenceCommands ? evidenceCommands[command.handler_ref] : undefined;
    const keyboardInjectionInvocationCount = measured
      ? measured.keyboard_injection_invocation_count
      : 0;
    const measuredFocusStealCount = measured ? measured.focus_steal_event_count : null;
    // The static forbidden-API hit count remains an independent signal; the
    // measured focus-steal count (live focus-audit handshake-owned events) is
    // an additional runtime invariant. Both must be zero for a PASS.
    const focusStealApiCount = forbidden_hits.length;
    const evidenceSource = measured ? "runtime_probe_measured" : "static_source_scan";

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
    if (requireRuntimeProbe && !measured) {
      violations.push({
        code: "RUNTIME_PROBE_EVIDENCE_REQUIRED",
        message: `${command.handler_ref} is missing runtime-probe evidence; runtime evidence is required for certifying automation-first audit runs (use --static-source-scan-ok only for explicit non-certifying scanner/report runs)`,
      });
    }
    if (measured && measured.keyboard_injection_invocation_count > 0) {
      violations.push({
        code: "RUNTIME_KEYBOARD_INJECTION_FIRED_COMMAND",
        message: `${command.handler_ref} fired ${measured.keyboard_injection_invocation_count} time(s) under the raw OS keyboard-injection probe (must be 0)`,
      });
    }
    if (measured && measured.focus_steal_event_count > 0) {
      violations.push({
        code: "RUNTIME_FOCUS_STEAL_OBSERVED",
        message: `${command.handler_ref} produced ${measured.focus_steal_event_count} handshake-owned foreground transition(s) under the live focus-audit probe (must be 0)`,
      });
    }

    return {
      command: command.handler_ref,
      file: command.file,
      ipc_callable: ipcCallable,
      foreground_exempt: foregroundExempt,
      keyboard_injection_invocation_count: keyboardInjectionInvocationCount,
      focus_steal_api_count: focusStealApiCount,
      runtime_focus_steal_event_count: measuredFocusStealCount,
      evidence_source: evidenceSource,
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

  const measuredCommandCount = commandReports.filter(
    (command) => command.evidence_source === "runtime_probe_measured",
  ).length;

  return {
    schema_id: "hsk.automation_first_audit@1",
    mode,
    certification_mode: requireRuntimeProbe
      ? "runtime_probe_required"
      : "static_source_scan_explicit",
    generated_at_utc: new Date().toISOString(),
    foreground_exempt_commands: [...FOREGROUND_EXEMPT_COMMANDS],
    command_count: commandReports.length,
    registered_handler_count: registered.length,
    runtime_probe: {
      required: requireRuntimeProbe,
      static_source_scan_allowed: !requireRuntimeProbe,
      evidence_present: Boolean(runtimeProbeEvidence),
      schema_id: runtimeProbeEvidence ? runtimeProbeEvidence.schema_id : null,
      platform: runtimeProbeEvidence ? runtimeProbeEvidence.platform : null,
      focus_audit_measured: runtimeProbeEvidence
        ? Boolean(runtimeProbeEvidence.focus_audit_measured)
        : false,
      keyboard_injection_measured: runtimeProbeEvidence
        ? Boolean(runtimeProbeEvidence.keyboard_injection_measured)
        : false,
      measured_command_count: measuredCommandCount,
    },
    commands: commandReports,
    violations,
    status: violations.length === 0 ? "PASS" : "FAIL",
  };
}

function renderMarkdown(audit) {
  const runtime = audit.runtime_probe || { evidence_present: false };
  const lines = [
    "# Automation-First Audit",
    "",
    `- status: ${audit.status}`,
    `- certification_mode: ${audit.certification_mode || "runtime_probe_required"}`,
    `- command_count: ${audit.command_count}`,
    `- foreground_exempt_commands: ${audit.foreground_exempt_commands.length}`,
    `- runtime_probe_evidence: ${runtime.evidence_present ? "present" : "absent (static source scan only)"}`,
    `- runtime_focus_audit_measured: ${runtime.focus_audit_measured ? "yes" : "no"}`,
    `- runtime_keyboard_injection_measured: ${runtime.keyboard_injection_measured ? "yes" : "no"}`,
    `- runtime_measured_command_count: ${runtime.measured_command_count || 0}`,
    "",
    "## Commands",
    "",
    "| command | status | ipc | focus/input hits | kbd_inject_count | runtime_focus_events | evidence |",
    "| --- | --- | --- | --- | --- | --- | --- |",
  ];

  for (const command of audit.commands) {
    const runtimeFocus = command.runtime_focus_steal_event_count == null
      ? "n/a"
      : command.runtime_focus_steal_event_count;
    lines.push(`| ${command.command} | ${command.status} | ${command.ipc_callable ? "yes" : "no"} | ${command.focus_steal_api_count} | ${command.keyboard_injection_invocation_count} | ${runtimeFocus} | ${command.evidence_source || "static_source_scan"} |`);
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
  const runtimeProbeEvidence = loadRuntimeProbeEvidence(args.runtimeProbeEvidence);
  const audit = args.fixture === "synthetic-violation"
    ? auditSyntheticViolation(runtimeProbeEvidence, args.requireRuntimeProbe)
    : auditRepo(resolveRepoRoot(args.repoRoot), runtimeProbeEvidence, args.requireRuntimeProbe);

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
