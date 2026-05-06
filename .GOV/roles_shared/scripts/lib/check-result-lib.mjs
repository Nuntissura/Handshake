import crypto from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
} from "./runtime-paths.mjs";

export const CHECK_RESULT_SCHEMA_ID = "hsk.check_result@1";
export const CHECK_RESULT_SCHEMA_VERSION = "check_result_v1";
export const CHECK_RESULT_VERDICTS = new Set(["OK", "WARN", "FAIL"]);
export const CHECK_RESULT_SUMMARY_MAX_CHARS = 120;
export const PHASE_BUNDLE_FAILURE_DOSSIER_SCHEMA_ID = "handshake.gov.phase_bundle_failure_dossier";
export const PHASE_BUNDLE_FAILURE_DOSSIER_SCHEMA_VERSION = "phase_bundle_failure_dossier_v1";

function normalizeVerdict(verdict) {
  const normalized = String(verdict || "").trim().toUpperCase();
  if (!CHECK_RESULT_VERDICTS.has(normalized)) {
    throw new Error(`createCheckResult: verdict must be one of ${[...CHECK_RESULT_VERDICTS].join(", ")}`);
  }
  return normalized;
}

function normalizeSummary(summary) {
  const normalized = String(summary || "").replace(/\s+/g, " ").trim();
  if (!normalized) {
    throw new Error("createCheckResult: summary is required");
  }
  if (normalized.length > CHECK_RESULT_SUMMARY_MAX_CHARS) {
    throw new Error(`createCheckResult: summary must be <= ${CHECK_RESULT_SUMMARY_MAX_CHARS} characters`);
  }
  return normalized;
}

export function compactCheckSummary(summary, maxChars = CHECK_RESULT_SUMMARY_MAX_CHARS) {
  const normalized = String(summary || "").replace(/\s+/g, " ").trim();
  const limit = Number(maxChars || CHECK_RESULT_SUMMARY_MAX_CHARS);
  if (normalized.length <= limit) return normalized;
  return `${normalized.slice(0, Math.max(0, limit - 3)).trimEnd()}...`;
}

function normalizeDetails(details) {
  if (!details || typeof details !== "object" || Array.isArray(details)) {
    throw new Error("createCheckResult: details must be an object");
  }
  return details;
}

function stableStringify(value) {
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function entryIdFor(entry) {
  return crypto.createHash("sha256").update(stableStringify({
    check: entry.check,
    wp_id: entry.wp_id,
    phase: entry.phase,
    verdict: entry.verdict,
    summary: entry.summary,
    details: entry.details,
  })).digest("hex").slice(0, 16);
}

export function createCheckResult({ verdict, summary, details = {} } = {}) {
  return {
    schema_id: CHECK_RESULT_SCHEMA_ID,
    schema_version: CHECK_RESULT_SCHEMA_VERSION,
    verdict: normalizeVerdict(verdict),
    summary: normalizeSummary(summary),
    details: normalizeDetails(details),
  };
}

export function formatCheckResultSummary(result) {
  const normalized = createCheckResult(result);
  return `${normalized.verdict} | ${normalized.summary}`;
}

export function formatVerboseCheckDetails(entry) {
  return JSON.stringify(entry, null, 2);
}

export function checkDetailsLogPath({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const root = path.resolve(runtimeRootAbs);
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) {
    return path.join(root, "check_details.jsonl");
  }
  return path.join(root, "roles_shared", "WP_COMMUNICATIONS", normalizedWpId, "check_details.jsonl");
}

export function appendCheckDetails({
  check,
  wpId = "",
  phase = "",
  result,
  timestamp = new Date().toISOString(),
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const normalizedCheck = String(check || "").trim();
  if (!normalizedCheck) {
    throw new Error("appendCheckDetails: check is required");
  }
  const normalizedResult = createCheckResult(result);
  const entry = {
    schema_id: "hsk.check_detail@1",
    schema_version: "check_detail_v1",
    timestamp,
    check: normalizedCheck,
    wp_id: String(wpId || "").trim() || null,
    phase: String(phase || "").trim() || null,
    verdict: normalizedResult.verdict,
    summary: normalizedResult.summary,
    details: normalizedResult.details,
  };
  entry.entry_id = entryIdFor(entry);

  const logPath = checkDetailsLogPath({ wpId, runtimeRootAbs });
  fs.mkdirSync(path.dirname(logPath), { recursive: true });
  if (fs.existsSync(logPath)) {
    const existing = fs.readFileSync(logPath, "utf8")
      .split(/\r?\n/)
      .filter(Boolean)
      .some((line) => {
        try {
          return JSON.parse(line).entry_id === entry.entry_id;
        } catch {
          return false;
        }
      });
    if (existing) {
      return { appended: false, logPath, entry };
    }
  }
  fs.appendFileSync(logPath, `${JSON.stringify(entry)}\n`, "utf8");
  return { appended: true, logPath, entry };
}

export function recordCheckResult({
  check,
  wpId = "",
  phase = "",
  verdict,
  summary,
  details = {},
  timestamp = new Date().toISOString(),
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const result = createCheckResult({
    verdict,
    summary: compactCheckSummary(summary),
    details,
  });
  const writeResult = appendCheckDetails({
    check,
    wpId,
    phase,
    result,
    timestamp,
    runtimeRootAbs,
  });
  return {
    result,
    writeResult,
    summaryLine: formatCheckResultSummary(result),
  };
}

function ensureTrailingNewline(value = "") {
  const text = String(value || "");
  return text.endsWith("\n") ? text : `${text}\n`;
}

function outputLines(value = "") {
  return String(value || "").split(/\r?\n/).filter((line) => line.length > 0);
}

function boundedText(value = "", maxChars = 2000) {
  const text = String(value || "");
  const limit = Number(maxChars || 2000);
  if (text.length <= limit) return text;
  return `${text.slice(0, Math.max(0, limit - 3))}...`;
}

export function failureDossierRootPath({ runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  return path.join(path.resolve(runtimeRootAbs), "roles_shared", "failure_dossiers");
}

export function wpDossierRootPath({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) return null;
  return path.join(path.resolve(runtimeRootAbs), "roles_shared", "WP_DOSSIERS", normalizedWpId);
}

export function wpDossierIndexPath({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const root = wpDossierRootPath({ wpId, runtimeRootAbs });
  return root ? path.join(root, "index.json") : null;
}

export function wpDossierEventsPath({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const root = wpDossierRootPath({ wpId, runtimeRootAbs });
  return root ? path.join(root, "events.jsonl") : null;
}

export function wpDossierArtifactManifestPath({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const root = wpDossierRootPath({ wpId, runtimeRootAbs });
  return root ? path.join(root, "artifact_manifest.json") : null;
}

export function wpDossierWorkflowPostmortemPath({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const root = wpDossierRootPath({ wpId, runtimeRootAbs });
  return root ? path.join(root, "workflow_postmortem.md") : null;
}

export function failureDossierLogPath({ runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS, wpId = "" } = {}) {
  const wpRoot = wpDossierRootPath({ wpId, runtimeRootAbs });
  if (wpRoot) return path.join(wpRoot, "bundle_failures", "phase_bundle_failures.jsonl");
  return path.join(failureDossierRootPath({ runtimeRootAbs }), "phase_bundle_failures.jsonl");
}

export function failureDossierMarkdownPath({ runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS, wpId = "" } = {}) {
  const wpRoot = wpDossierRootPath({ wpId, runtimeRootAbs });
  if (wpRoot) return path.join(wpRoot, "bundle_failures", "phase_bundle_failures.md");
  return path.join(failureDossierRootPath({ runtimeRootAbs }), "phase_bundle_failures.md");
}

function envSummary(env = process.env) {
  const keys = [
    "HANDSHAKE_ACTIVE_REPO_ROOT",
    "HANDSHAKE_GOV_ROOT",
    "HANDSHAKE_GOV_RUNTIME_ROOT",
    "HANDSHAKE_RUNTIME_ROOT",
    "HANDSHAKE_ARTIFACT_ROOT",
  ];
  return Object.fromEntries(keys.map((key) => [key, String(env?.[key] || "")]));
}

function writeDossierArtifact({ runId, label, content = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS, wpId = "" } = {}) {
  const text = String(content || "");
  if (!text) return null;
  const wpRoot = wpDossierRootPath({ wpId, runtimeRootAbs });
  const artifactDir = wpRoot
    ? path.join(wpRoot, "raw", "commands")
    : path.join(failureDossierRootPath({ runtimeRootAbs }), "artifacts");
  fs.mkdirSync(artifactDir, { recursive: true });
  const artifactPath = path.join(artifactDir, `${runId}-${label}.txt`);
  fs.writeFileSync(artifactPath, text, "utf8");
  return artifactPath;
}

function renderFailureDossierMarkdown(entries = []) {
  const lines = [
    "# Phase Bundle Failure Dossier",
    "",
    "Machine projection of recent phase-bundle failures. JSONL is authoritative.",
    "",
  ];
  for (const entry of entries.slice(-50)) {
    lines.push(`## ${entry.timestamp} / ${entry.bundle} / ${entry.substep_id}`);
    lines.push("");
    lines.push(`- Run ID: ${entry.run_id}`);
    lines.push(`- Phase: ${entry.phase || "NONE"}`);
    lines.push(`- Owner role: ${entry.owner_role || "UNKNOWN"}`);
    lines.push(`- Side effect class: ${entry.side_effect_class || "UNKNOWN"}`);
    lines.push(`- Exit code: ${entry.exit_code}`);
    lines.push(`- Duration ms: ${entry.duration_ms}`);
    lines.push(`- Debug artifact: ${entry.debug_artifact || "NONE"}`);
    lines.push(`- Suspected cause: ${entry.suspected_cause_category || "UNKNOWN"}`);
    lines.push(`- Remediation hint: ${entry.remediation_hint || "Inspect the JSONL row and linked artifacts."}`);
    lines.push("");
  }
  return `${lines.join("\n")}\n`;
}

function refreshFailureDossierMarkdown({ runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS, wpId = "" } = {}) {
  const logPath = failureDossierLogPath({ runtimeRootAbs, wpId });
  const mdPath = failureDossierMarkdownPath({ runtimeRootAbs, wpId });
  if (!fs.existsSync(logPath)) return;
  const entries = fs.readFileSync(logPath, "utf8")
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  fs.writeFileSync(mdPath, renderFailureDossierMarkdown(entries), "utf8");
}

function ensureWpDossierFolders({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const root = wpDossierRootPath({ wpId, runtimeRootAbs });
  if (!root) return null;
  for (const rel of ["raw", "acp", "repomem", "commands", "bundle_failures", "raw/commands"]) {
    fs.mkdirSync(path.join(root, rel), { recursive: true });
  }
  return root;
}

function runtimeRelative(runtimeRootAbs, targetPath) {
  if (!targetPath) return null;
  return path.relative(path.resolve(runtimeRootAbs), path.resolve(targetPath)).replace(/\\/g, "/");
}

function writeWpDossierIndex({
  wpId = "",
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
  latestFailure = null,
} = {}) {
  const root = ensureWpDossierFolders({ wpId, runtimeRootAbs });
  if (!root) return null;
  const logPath = failureDossierLogPath({ runtimeRootAbs, wpId });
  const eventsPath = wpDossierEventsPath({ wpId, runtimeRootAbs });
  const manifestPath = wpDossierArtifactManifestPath({ wpId, runtimeRootAbs });
  const postmortemPath = wpDossierWorkflowPostmortemPath({ wpId, runtimeRootAbs });
  const failureRows = fs.existsSync(logPath)
    ? fs.readFileSync(logPath, "utf8").split(/\r?\n/).filter(Boolean).length
    : 0;
  const index = {
    schema_id: "handshake.gov.wp_dossier_index",
    schema_version: "wp_dossier_index_v1",
    wp_id: String(wpId || "").trim(),
    updated_at_utc: new Date().toISOString(),
    root,
    contract: {
      raw_archive_policy: "dump_all_logs_for_posterity",
      model_entrypoint: "index.json",
      terminal_narrative: "workflow_postmortem.md",
      storage: "external_governance_runtime_root",
    },
    paths: {
      index: runtimeRelative(runtimeRootAbs, wpDossierIndexPath({ wpId, runtimeRootAbs })),
      events_jsonl: runtimeRelative(runtimeRootAbs, eventsPath),
      artifact_manifest_json: runtimeRelative(runtimeRootAbs, manifestPath),
      workflow_postmortem_md: runtimeRelative(runtimeRootAbs, postmortemPath),
      raw_dir: runtimeRelative(runtimeRootAbs, path.join(root, "raw")),
      acp_dir: runtimeRelative(runtimeRootAbs, path.join(root, "acp")),
      repomem_dir: runtimeRelative(runtimeRootAbs, path.join(root, "repomem")),
      commands_dir: runtimeRelative(runtimeRootAbs, path.join(root, "commands")),
      bundle_failures_dir: runtimeRelative(runtimeRootAbs, path.join(root, "bundle_failures")),
      bundle_failures_jsonl: runtimeRelative(runtimeRootAbs, logPath),
      bundle_failures_md: runtimeRelative(runtimeRootAbs, failureDossierMarkdownPath({ runtimeRootAbs, wpId })),
    },
    counts: {
      bundle_failure_entries: failureRows,
    },
    latest_failure: latestFailure,
  };
  fs.writeFileSync(wpDossierIndexPath({ wpId, runtimeRootAbs }), `${JSON.stringify(index, null, 2)}\n`, "utf8");

  const artifactRefs = latestFailure?.artifact_refs || [];
  const manifest = {
    schema_id: "handshake.gov.wp_dossier_artifact_manifest",
    schema_version: "wp_dossier_artifact_manifest_v1",
    wp_id: String(wpId || "").trim(),
    updated_at_utc: index.updated_at_utc,
    artifacts: artifactRefs,
  };
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

  const event = {
    schema_id: "handshake.gov.wp_dossier_event",
    schema_version: "wp_dossier_event_v1",
    timestamp: index.updated_at_utc,
    wp_id: String(wpId || "").trim(),
    event_type: latestFailure ? "BUNDLE_FAILURE_RECORDED" : "DOSSIER_INDEX_REFRESHED",
    run_id: latestFailure?.run_id || null,
    entry_id: latestFailure?.entry_id || null,
  };
  fs.appendFileSync(eventsPath, `${JSON.stringify(event)}\n`, "utf8");
  return index;
}

export function appendFailureDossierEntry({
  timestamp = new Date().toISOString(),
  phase = "",
  bundle = "",
  substepId = "",
  command = [],
  ownerRole = "",
  sideEffectClass = "",
  cwd = process.cwd(),
  env = process.env,
  exitCode = 1,
  signal = null,
  durationMs = 0,
  stdout = "",
  stderr = "",
  debugArtifact = "",
  invariant = "",
  suspectedCauseCategory = "CHECK_FAILURE",
  remediationHint = "Inspect the linked stdout/stderr artifacts and the related topology rows.",
  relatedTopologyRows = [],
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
  wpId = "",
} = {}) {
  const normalizedBundle = String(bundle || "phase-bundle").trim();
  const normalizedSubstep = String(substepId || "unknown-substep").trim();
  const runId = `${timestamp.replace(/[-:.TZ]/g, "").slice(0, 14)}-${crypto.createHash("sha256").update(`${normalizedBundle}:${normalizedSubstep}:${stdout}:${stderr}`).digest("hex").slice(0, 12)}`;
  const root = wpDossierRootPath({ wpId, runtimeRootAbs }) || failureDossierRootPath({ runtimeRootAbs });
  fs.mkdirSync(root, { recursive: true });
  const stdoutArtifact = writeDossierArtifact({ runId, label: "stdout", content: stdout, runtimeRootAbs, wpId });
  const stderrArtifact = writeDossierArtifact({ runId, label: "stderr", content: stderr, runtimeRootAbs, wpId });
  const artifactRefs = [
    stdoutArtifact ? { kind: "stdout", path: stdoutArtifact } : null,
    stderrArtifact ? { kind: "stderr", path: stderrArtifact } : null,
  ].filter(Boolean);
  const entry = {
    schema_id: PHASE_BUNDLE_FAILURE_DOSSIER_SCHEMA_ID,
    schema_version: PHASE_BUNDLE_FAILURE_DOSSIER_SCHEMA_VERSION,
    run_id: runId,
    timestamp,
    wp_id: String(wpId || "").trim() || null,
    phase: String(phase || "").trim() || null,
    bundle: normalizedBundle,
    substep_id: normalizedSubstep,
    command: Array.isArray(command) ? command.map((part) => String(part)) : [String(command || "")],
    owner_role: String(ownerRole || "").trim() || null,
    side_effect_class: String(sideEffectClass || "").trim() || null,
    cwd: path.resolve(cwd || process.cwd()),
    env_summary: envSummary(env),
    exit_code: Number(exitCode),
    signal: signal || null,
    duration_ms: Number(durationMs || 0),
    stdout_artifact: stdoutArtifact,
    stderr_artifact: stderrArtifact,
    artifact_refs: artifactRefs,
    wp_dossier_root: wpDossierRootPath({ wpId, runtimeRootAbs }),
    wp_dossier_index: wpDossierIndexPath({ wpId, runtimeRootAbs }),
    workflow_postmortem_ref: wpDossierWorkflowPostmortemPath({ wpId, runtimeRootAbs }),
    stdout_excerpt: boundedText(stdout),
    stderr_excerpt: boundedText(stderr),
    debug_artifact: String(debugArtifact || "").trim() || null,
    invariant: String(invariant || "").trim() || null,
    suspected_cause_category: String(suspectedCauseCategory || "").trim() || "CHECK_FAILURE",
    remediation_hint: String(remediationHint || "").trim() || null,
    related_topology_rows: Array.isArray(relatedTopologyRows) ? relatedTopologyRows.map((row) => String(row)) : [],
  };
  entry.entry_id = crypto.createHash("sha256").update(JSON.stringify(entry)).digest("hex").slice(0, 16);
  fs.mkdirSync(path.dirname(failureDossierLogPath({ runtimeRootAbs, wpId })), { recursive: true });
  fs.appendFileSync(failureDossierLogPath({ runtimeRootAbs, wpId }), `${JSON.stringify(entry)}\n`, "utf8");
  refreshFailureDossierMarkdown({ runtimeRootAbs, wpId });
  if (String(wpId || "").trim()) {
    writeWpDossierIndex({
      wpId,
      runtimeRootAbs,
      latestFailure: {
        run_id: runId,
        entry_id: entry.entry_id,
        bundle: entry.bundle,
        substep_id: entry.substep_id,
        artifact_refs: artifactRefs.map((artifact) => ({
          ...artifact,
          path: runtimeRelative(runtimeRootAbs, artifact.path),
        })),
      },
    });
  }
  return {
    entry_id: entry.entry_id,
    run_id: runId,
    log_path: failureDossierLogPath({ runtimeRootAbs, wpId }),
    markdown_path: failureDossierMarkdownPath({ runtimeRootAbs, wpId }),
    wp_dossier_index: wpDossierIndexPath({ wpId, runtimeRootAbs }),
    stdout_artifact: stdoutArtifact,
    stderr_artifact: stderrArtifact,
  };
}

export function runSubprocessCheckStep({
  check,
  scriptPath,
  args = [],
  cwd = process.cwd(),
  env = process.env,
  wpId = "",
  phase = "",
  bundle = "",
  ownerRole = "",
  sideEffectClass = "",
  invariant = "",
  suspectedCauseCategory = "CHECK_FAILURE",
  remediationHint = "",
  relatedTopologyRows = [],
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const normalizedCheck = String(check || "").trim();
  if (!normalizedCheck) {
    throw new Error("runSubprocessCheckStep: check is required");
  }
  const normalizedScriptPath = String(scriptPath || "").trim();
  if (!normalizedScriptPath) {
    throw new Error("runSubprocessCheckStep: scriptPath is required");
  }

  const startedAt = Date.now();
  const result = spawnSync(process.execPath, [normalizedScriptPath, ...args], {
    cwd,
    env,
    encoding: "utf8",
  });
  const durationMs = Date.now() - startedAt;
  const status = result.status ?? 1;
  const stdout = String(result.stdout || "");
  const stderr = String(result.stderr || "");
  const output = ensureTrailingNewline(`${stdout}${stderr}`.trimEnd());
  const ok = status === 0;
  let failureDossier = null;
  if (!ok) {
    try {
      failureDossier = appendFailureDossierEntry({
        phase,
        bundle: bundle || "gov-check",
        substepId: normalizedCheck,
        command: [process.execPath, normalizedScriptPath, ...args],
        ownerRole,
        sideEffectClass,
        cwd,
        env,
        exitCode: status,
        signal: result.signal || null,
        durationMs,
        stdout,
        stderr,
        debugArtifact: checkDetailsLogPath({ wpId, runtimeRootAbs }),
        invariant,
        suspectedCauseCategory,
        remediationHint,
        relatedTopologyRows,
        runtimeRootAbs,
        wpId,
      });
    } catch (error) {
      failureDossier = {
        error: String(error?.message || error || ""),
      };
    }
  }
  const recorded = recordCheckResult({
    check: normalizedCheck,
    wpId,
    phase,
    verdict: ok ? "OK" : "FAIL",
    summary: `${normalizedCheck} ${ok ? "ok" : "failed"}`,
    details: {
      script_path: normalizedScriptPath,
      args: args.map((arg) => String(arg)),
      cwd: path.resolve(cwd || process.cwd()),
      exit_status: status,
      signal: result.signal || null,
      duration_ms: durationMs,
      stdout,
      stderr,
      output_lines: outputLines(output),
      error: result.error ? String(result.error?.message || result.error) : null,
      failure_dossier: failureDossier,
    },
    runtimeRootAbs,
  });

  return {
    ok,
    status,
    signal: result.signal || null,
    stdout,
    stderr,
    output,
    ...recorded,
  };
}

export function readCheckDetails({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS, limit = 50 } = {}) {
  const logPath = checkDetailsLogPath({ wpId, runtimeRootAbs });
  if (!fs.existsSync(logPath)) {
    return [];
  }
  const rows = fs.readFileSync(logPath, "utf8")
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  return rows.slice(Math.max(0, rows.length - Number(limit || 50)));
}
