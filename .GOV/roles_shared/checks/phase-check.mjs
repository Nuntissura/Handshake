#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { GOV_ROOT_ABS, GOV_ROOT_REPO_REL, REPO_ROOT, repoPathAbs, resolveWorkPacketPath } from "../scripts/lib/runtime-paths.mjs";
import { compactGateOutputSummary, writeGateOutputArtifact } from "../scripts/lib/gate-output-artifact-lib.mjs";
import { ensureWpCommunications } from "../scripts/wp/ensure-wp-communications.mjs";
import { captureCheckFindings } from "../scripts/memory/memory-capture-from-check.mjs";
import { buildActiveLaneBrief, formatActiveLaneBrief } from "../scripts/session/active-lane-brief-lib.mjs";
import { loadSessionRegistry } from "../scripts/session/session-registry-lib.mjs";
import {
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorWorktreeDir,
} from "../scripts/session/session-policy.mjs";
import {
  buildWpCommunicationHealthCheckResult,
  formatWpCommunicationHealthCheckResult,
} from "./wp-communication-health-check.mjs";
import { buildPhaseCheckCommand, buildPhaseCheckPlan } from "./phase-check-lib.mjs";
import {
  buildIntegrationValidatorContextBriefFromEnvironment,
  formatIntegrationValidatorContextBrief,
} from "../../roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs";
import {
  buildIntegrationValidatorCloseoutCheckResult,
  formatIntegrationValidatorCloseoutCheckResult,
  loadDeclaredRuntimeStatus,
} from "../../roles/validator/scripts/lib/integration-validator-closeout-lib.mjs";
import {
  buildValidatorPacketCompleteResult,
  formatValidatorPacketCompleteResult,
  buildValidatorHandoffCheckResult,
  formatValidatorHandoffCheckResult,
} from "../../roles/validator/scripts/lib/validator-governance-lib.mjs";
import { parseMergeProgressionTruth } from "../scripts/lib/merge-progression-truth-lib.mjs";
import {
  inferValidationVerdictFromPublication,
  parseExecutionCloseoutMode,
  readExecutionPublicationView,
} from "../scripts/lib/wp-execution-state-lib.mjs";
import { findOpenWorkflowDossierPath } from "../scripts/audit/workflow-dossier-lib.mjs";
import {
  buildWpMetrics,
  buildWpTimelineEntries,
  buildWpTimelineSpans,
  buildWpTimelineSummary,
  loadWpTimelineArtifacts,
} from "../scripts/session/wp-timeline-lib.mjs";

export { buildPhaseCheckCommand, buildPhaseCheckPlan, PHASE_VALUES } from "./phase-check-lib.mjs";

const CLOSEOUT_SYNC_SHA_RE = /^[0-9a-f]{7,40}$/i;
const CLOSEOUT_SYNC_FLAG_SET = new Set(["--sync-mode", "--context", "--merged-main-sha", "--sync-debug"]);
const TERMINAL_READY_SESSION_CLOSE_ROLE_VALUES = new Set([
  "ACTIVATION_MANAGER",
  "CODER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "MEMORY_MANAGER",
]);

export function resolvePhaseCheckCwd() {
  const injectedRepoRoot = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  return path.resolve(injectedRepoRoot || REPO_ROOT);
}

export function resolveCloseoutSyncCwd({
  wpId = "",
  phaseCheckCwd = resolvePhaseCheckCwd(),
  registrySessions = null,
} = {}) {
  const defaultCwd = path.resolve(phaseCheckCwd || resolvePhaseCheckCwd());
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) return defaultCwd;

  const sessions = Array.isArray(registrySessions)
    ? registrySessions
    : loadSessionRegistry(defaultCwd).registry.sessions || [];
  const preferredSessionKey = `INTEGRATION_VALIDATOR:${normalizedWpId}`;
  const integrationValidatorSession = sessions.find((session) =>
    String(session?.wp_id || "").trim() === normalizedWpId
    && String(session?.role || "").trim().toUpperCase() === "INTEGRATION_VALIDATOR"
    && String(session?.session_key || "").trim() === preferredSessionKey
    && String(session?.local_worktree_dir || "").trim()
  ) || sessions.find((session) =>
    String(session?.wp_id || "").trim() === normalizedWpId
    && String(session?.role || "").trim().toUpperCase() === "INTEGRATION_VALIDATOR"
    && String(session?.local_worktree_dir || "").trim()
  );

  const targetWorktreeDir = String(
    integrationValidatorSession?.local_worktree_dir
    || defaultIntegrationValidatorWorktreeDir(normalizedWpId)
    || "",
  ).trim();
  return targetWorktreeDir ? path.resolve(REPO_ROOT, targetWorktreeDir) : defaultCwd;
}

function currentGitContextAt(cwd = resolvePhaseCheckCwd()) {
  const resolvedCwd = path.resolve(cwd || resolvePhaseCheckCwd());
  const runGit = (args = []) => {
    const result = spawnSync("git", args, {
      cwd: resolvedCwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    return result.status === 0 ? String(result.stdout || "").trim() : "";
  };
  return {
    branch: runGit(["rev-parse", "--abbrev-ref", "HEAD"]),
    topLevel: runGit(["rev-parse", "--show-toplevel"]),
    statusShort: runGit(["status", "-sb"]),
    statusPorcelain: runGit(["status", "--porcelain=v1"]),
  };
}

function printUsage(message = "") {
  if (message) console.error(`[PHASE_CHECK] ${message}`);
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles_shared/checks/phase-check.mjs <STARTUP|HANDOFF|VERDICT|CLOSEOUT> WP-{ID} [ROLE] [SESSION] [args...]`);
}

function ensureTrailingNewline(value = "") {
  const text = String(value || "");
  return text.endsWith("\n") ? text : `${text}\n`;
}

export function parseMarkersFromContent(content) {
  const lines = String(content || "").split("\n");
  let inCodeFence = false;
  const result = {
    bootstrapHeadingLine: -1,
    skeletonHeadingLine: -1,
    implementationDetected: false,
    statusInProgress: false,
  };

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const trimmed = line.trim();
    if (trimmed.startsWith("```")) {
      inCodeFence = !inCodeFence;
      continue;
    }
    if (inCodeFence) continue;
    if (/^#{1,6}\s+BOOTSTRAP\b/i.test(line) && result.bootstrapHeadingLine === -1) {
      result.bootstrapHeadingLine = index;
    }
    if (/^#{1,6}\s+SKELETON\b/i.test(line) && result.skeletonHeadingLine === -1) {
      result.skeletonHeadingLine = index;
    }
    if (
      /^#{1,6}\s+VALIDATION\s*\(Coder\)/i.test(line)
      || /^#{1,6}\s+VALIDATION REPORT\b/i.test(line)
    ) {
      result.implementationDetected = true;
    }
    if (/Status:\s*In[- ]?Progress/i.test(line)) {
      result.statusInProgress = true;
    }
  }

  return result;
}

export function runGateCheck(wpId) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) {
    return {
      ok: false,
      output: "GATE FAIL: WP_ID is required.\n",
    };
  }

  const resolved = resolveWorkPacketPath(normalizedWpId);
  const wpPath = resolved?.packetPath
    ? repoPathAbs(resolved.packetPath)
    : repoPathAbs(`${GOV_ROOT_REPO_REL}/task_packets/${normalizedWpId}.md`);
  if (!fs.existsSync(wpPath)) {
    return {
      ok: false,
      output: `? GATE FAIL: Work Packet ${normalizedWpId} not found at the resolved packet path.\n`,
    };
  }

  const content = fs.readFileSync(wpPath, "utf8");
  const parsed = parseMarkersFromContent(content);
  const lines = [`Checking Phase Gate for ${normalizedWpId}...`];

  if (parsed.statusInProgress && parsed.bootstrapHeadingLine === -1) {
    lines.push("? GATE FAIL: 'In Progress' status requires a BOOTSTRAP block.");
    return { ok: false, output: `${lines.join("\n")}\n` };
  }

  const missingPhases = [];
  if (parsed.bootstrapHeadingLine === -1) missingPhases.push("BOOTSTRAP");
  if (parsed.skeletonHeadingLine === -1) missingPhases.push("SKELETON");
  if (missingPhases.length > 0 && parsed.implementationDetected) {
    lines.push(`? GATE FAIL: Missing mandatory phases: ${missingPhases.join(", ")}`);
    return { ok: false, output: `${lines.join("\n")}\n` };
  }
  if (parsed.bootstrapHeadingLine === -1 || parsed.skeletonHeadingLine === -1) {
    lines.push("? GATE FAIL: Missing BOOTSTRAP or SKELETON markers.");
    return { ok: false, output: `${lines.join("\n")}\n` };
  }
  if (parsed.bootstrapHeadingLine > parsed.skeletonHeadingLine) {
    lines.push("? GATE FAIL: SKELETON appears before BOOTSTRAP.");
    return { ok: false, output: `${lines.join("\n")}\n` };
  }

  lines.push("? GATE PASS: Workflow sequence verified.");
  return { ok: true, output: `${lines.join("\n")}\n` };
}

function runSubprocessStep({ scriptPath: targetScriptPath, args = [] }) {
  const result = spawnSync(process.execPath, [targetScriptPath, ...args], {
    encoding: "utf8",
  });
  return {
    ok: result.status === 0,
    output: ensureTrailingNewline(`${result.stdout || ""}${result.stderr || ""}`.trimEnd()),
  };
}

function parseCloseoutSyncMode(rawMode = "") {
  const modeSpec = parseExecutionCloseoutMode(rawMode);
  if (!modeSpec) return null;
  return {
    mode: modeSpec.mode,
    requireMergedMainCommit: modeSpec.require_merged_main_commit,
  };
}

function hasCloseoutSyncFlags(args = []) {
  return (args || []).some((value) => CLOSEOUT_SYNC_FLAG_SET.has(String(value || "").trim()));
}

function consumeFlagValueParts(tokens = [], startIndex = 0) {
  const valueParts = [];
  let index = startIndex;
  while (index + 1 < tokens.length && !String(tokens[index + 1] || "").trim().startsWith("--")) {
    index += 1;
    valueParts.push(String(tokens[index] || ""));
  }
  return {
    value: valueParts.join(" ").trim(),
    nextIndex: index,
  };
}

export function parseCloseoutSyncOptions(args = []) {
  const tokens = Array.isArray(args) ? args.map((value) => String(value || "")).filter((value) => value !== "") : [];
  let rawMode = "";
  let context = "";
  let mergedMainSha = "";
  let debug = false;

  for (let index = 0; index < tokens.length; index += 1) {
    const token = String(tokens[index] || "").trim();
    if (token === "--sync-debug") {
      debug = true;
      continue;
    }
    if (token === "--sync-mode" || token === "--context" || token === "--merged-main-sha") {
      const consumed = consumeFlagValueParts(tokens, index);
      if (!consumed.value) {
        throw new Error(`${token} requires a value`);
      }
      if (token === "--sync-mode") rawMode = consumed.value;
      if (token === "--context") context = consumed.value;
      if (token === "--merged-main-sha") mergedMainSha = consumed.value;
      index = consumed.nextIndex;
      continue;
    }
  }

  const modeSpec = rawMode ? parseCloseoutSyncMode(rawMode) : null;
  if (rawMode && !modeSpec) {
    throw new Error("CLOSEOUT --sync-mode must be MERGE_PENDING, CONTAINED_IN_MAIN, FAIL, OUTDATED_ONLY, or ABANDONED");
  }
  if (!modeSpec) {
    if (context || mergedMainSha || debug) {
      throw new Error("CLOSEOUT sync flags require --sync-mode");
    }
    return {
      modeSpec: null,
      context: "",
      mergedMainSha: "",
      debug: false,
    };
  }
  if (String(context || "").trim().length < 40) {
    throw new Error("CLOSEOUT --context must be at least 40 characters when --sync-mode is used");
  }
  if (modeSpec.requireMergedMainCommit) {
    if (!CLOSEOUT_SYNC_SHA_RE.test(String(mergedMainSha || "").trim())) {
      throw new Error("CLOSEOUT --merged-main-sha must be provided for --sync-mode CONTAINED_IN_MAIN");
    }
  } else if (String(mergedMainSha || "").trim()) {
    throw new Error("--merged-main-sha is only valid for --sync-mode CONTAINED_IN_MAIN");
  }
  return {
    modeSpec,
    context: String(context || "").trim(),
    mergedMainSha: String(mergedMainSha || "").trim(),
    debug,
  };
}

export function parseCommittedTargetArgs(args = []) {
  const tokens = Array.isArray(args) ? args.map((value) => String(value || "")).filter((value) => value !== "") : [];
  let rev = "";
  let range = "";

  for (let index = 0; index < tokens.length; index += 1) {
    const token = String(tokens[index] || "").trim();
    if (token === "--rev" || token === "--range") {
      const nextValue = String(tokens[index + 1] || "");
      if (!nextValue || String(nextValue).trim().startsWith("--")) {
        continue;
      }
      if (token === "--rev") rev = nextValue;
      if (token === "--range") range = nextValue;
      index += 1;
    }
  }

  return {
    rev: String(rev || "").trim(),
    range: String(range || "").trim(),
  };
}

function runCloseoutSyncStep({ wpId = "", syncOptions = {} } = {}) {
  const mode = String(syncOptions?.modeSpec?.mode || "").trim();
  if (!wpId || !mode) {
    return {
      ok: false,
      output: "CLOSEOUT sync requires WP_ID and sync mode.\n",
    };
  }
  const repomemScript = repoPathAbs(`${GOV_ROOT_REPO_REL}/roles_shared/scripts/memory/repomem.mjs`);
  const closeoutSyncScript = repoPathAbs(`${GOV_ROOT_REPO_REL}/roles/validator/scripts/integration-validator-closeout-sync.mjs`);
  const triggerRef = `phase-check CLOSEOUT ${wpId} --sync-mode ${mode}`;
  const phaseCheckCwd = resolvePhaseCheckCwd();
  const closeoutSyncCwd = resolveCloseoutSyncCwd({
    wpId,
    phaseCheckCwd,
  });

  // RGF-183: fail-fast when closeout resolves to the kernel root for a product-contained WP.
  // If resolveCloseoutSyncCwd fell back to phaseCheckCwd (the kernel), the signed-scope
  // validation will use kernel git context and produce false missing-file/drift failures.
  const normalizedCloseoutCwd = path.resolve(closeoutSyncCwd).replace(/\\/g, "/").toLowerCase();
  const normalizedKernelCwd = path.resolve(phaseCheckCwd).replace(/\\/g, "/").toLowerCase();
  if (normalizedCloseoutCwd === normalizedKernelCwd) {
    const packetInfo = resolveWorkPacketPath(wpId);
    const packetText = packetInfo?.packetAbsPath && fs.existsSync(packetInfo.packetAbsPath)
      ? fs.readFileSync(packetInfo.packetAbsPath, "utf8")
      : "";
    const prepareWorktreeDir = (packetText.match(/^\s*-\s*\**PREPARE_WORKTREE_DIR\**\s*:\s*(.+)/mi) || [])[1]?.trim() || "";
    const intValWorktreeDir = (packetText.match(/^\s*-\s*\**INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR\**\s*:\s*(.+)/mi) || [])[1]?.trim() || "";
    const declaredProductWorktree = intValWorktreeDir || prepareWorktreeDir;
    if (declaredProductWorktree && declaredProductWorktree !== ".") {
      return {
        ok: false,
        output: [
          `[RGF-183] CLOSEOUT sync failed: resolved to kernel root instead of product worktree.`,
          `  kernel_root: ${phaseCheckCwd}`,
          `  declared_product_worktree: ${declaredProductWorktree}`,
          `  resolved_closeout_cwd: ${closeoutSyncCwd}`,
          `  The WP's committed target lives in a product worktree, but closeout resolved to the kernel.`,
          `  Register an INTEGRATION_VALIDATOR session with local_worktree_dir pointing to the product worktree,`,
          `  or run phase-check CLOSEOUT from the product worktree with HANDSHAKE_ACTIVE_REPO_ROOT set.`,
        ].join("\n") + "\n",
      };
    }
  }

  const outputChunks = [];
  const repomemGateResult = spawnSync(process.execPath, [repomemScript, "gate"], {
    cwd: phaseCheckCwd,
    encoding: "utf8",
  });
  outputChunks.push(ensureTrailingNewline(`${repomemGateResult.stdout || ""}${repomemGateResult.stderr || ""}`.trimEnd()));
  if (repomemGateResult.status !== 0) {
    return {
      ok: false,
      output: outputChunks.join(""),
    };
  }
  const repomemContextArgs = [
    repomemScript,
    "context",
    String(syncOptions.context || "").trim(),
    "--trigger",
    triggerRef,
    "--wp",
    wpId,
  ];
  const repomemContextResult = spawnSync(process.execPath, repomemContextArgs, {
    cwd: phaseCheckCwd,
    encoding: "utf8",
  });
  outputChunks.push(ensureTrailingNewline(`${repomemContextResult.stdout || ""}${repomemContextResult.stderr || ""}`.trimEnd()));
  if (repomemContextResult.status !== 0) {
    return {
      ok: false,
      output: outputChunks.join(""),
    };
  }
  const commandArgs = [
    closeoutSyncScript,
    wpId,
    mode,
  ];
  if (syncOptions?.modeSpec?.requireMergedMainCommit) {
    commandArgs.push(String(syncOptions.mergedMainSha || "").trim());
  }
  if (syncOptions?.debug) {
    commandArgs.push("--debug");
  }
  const result = spawnSync(process.execPath, commandArgs, {
    cwd: closeoutSyncCwd,
    encoding: "utf8",
    env: {
      ...process.env,
      HANDSHAKE_GOV_ROOT: GOV_ROOT_ABS,
      HANDSHAKE_ACTIVE_REPO_ROOT: closeoutSyncCwd,
    },
  });
  outputChunks.push(ensureTrailingNewline(`${result.stdout || ""}${result.stderr || ""}`.trimEnd()));
  return {
    ok: result.status === 0,
    output: outputChunks.join(""),
  };
}

export function resolveTerminalReadySessionsForWp({
  wpId = "",
  registrySessions = [],
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  return (Array.isArray(registrySessions) ? registrySessions : [])
    .filter((session) => String(session?.wp_id || "").trim() === normalizedWpId)
    .filter((session) => TERMINAL_READY_SESSION_CLOSE_ROLE_VALUES.has(String(session?.role || "").trim().toUpperCase()))
    .filter((session) => String(session?.runtime_state || "").trim().toUpperCase() === "READY");
}

function runTerminalReadySessionCloseStep({ wpId = "" } = {}) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) {
    return {
      ok: false,
      output: "Terminal session cleanup requires WP_ID.\n",
    };
  }

  const registrySessions = loadSessionRegistry(resolvePhaseCheckCwd()).registry.sessions || [];
  const sessionsToClose = resolveTerminalReadySessionsForWp({
    wpId: normalizedWpId,
    registrySessions,
  });
  if (sessionsToClose.length === 0) {
    return {
      ok: true,
      output: "[TERMINAL_SESSION_CLEANUP] PASS: no stale READY governed sessions remain.\n",
    };
  }

  const closeScript = repoPathAbs(`${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/session-control-command.mjs`);
  const lines = [
    `[TERMINAL_SESSION_CLEANUP] ${sessionsToClose.length} terminal READY session(s) will be closed.`,
  ];

  for (const session of sessionsToClose) {
    const role = String(session?.role || "").trim().toUpperCase();
    const result = spawnSync(process.execPath, [closeScript, "CLOSE_SESSION", role, normalizedWpId], {
      cwd: resolvePhaseCheckCwd(),
      encoding: "utf8",
      env: process.env,
    });
    const rendered = ensureTrailingNewline(`${result.stdout || ""}${result.stderr || ""}`.trimEnd());
    lines.push(`- ${role}: ${result.status === 0 ? "PASS" : "FAIL"}`);
    for (const line of rendered.trimEnd().split("\n")) {
      if (line.trim()) lines.push(`  ${line}`);
    }
    if (result.status !== 0) {
      return {
        ok: false,
        output: `${lines.join("\n")}\n`,
      };
    }
  }

  return {
    ok: true,
    output: `${lines.join("\n")}\n`,
  };
}

function runWorkflowDossierCloseoutStep({
  wpId = "",
  ok = true,
  syncOptions = {},
  why = "",
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) {
    return {
      ok: false,
      output: "Workflow dossier closeout append requires WP_ID.\n",
    };
  }

  const governanceRepoRoot = path.resolve(GOV_ROOT_ABS, "..");
  const existingDossierPath = findOpenWorkflowDossierPath(governanceRepoRoot, normalizedWpId);
  if (!existingDossierPath) {
    return {
      ok: true,
      output: "[WORKFLOW_DOSSIER_CLOSEOUT] SKIP: no open Workflow Dossier found for this WP.\n",
    };
  }

  const workflowDossierScript = path.resolve(GOV_ROOT_ABS, "roles_shared", "scripts", "audit", "workflow-dossier.mjs");
  const phaseCheckCwd = governanceRepoRoot;
  const syncMode = String(syncOptions?.modeSpec?.mode || "").trim().toUpperCase() || "NONE";
  const outputLines = [];

  const noteSummary = [
    `phase-check CLOSEOUT result=${ok ? "PASS" : "FAIL"}`,
    `sync_mode=${syncMode}`,
    `why=${String(why || "").trim() || "NONE"}`,
  ].join(" | ");

  const noteResult = spawnSync(process.execPath, [
    workflowDossierScript,
    "note",
    normalizedWpId,
    "EXECUTION",
    noteSummary,
    "--role",
    "INTEGRATION_VALIDATOR",
    "--tag",
    "CLOSEOUT_GATE",
    "--surface",
    "phase-check CLOSEOUT",
    "--file",
    existingDossierPath,
  ], {
    cwd: phaseCheckCwd,
    encoding: "utf8",
    env: process.env,
  });
  outputLines.push(`[WORKFLOW_DOSSIER_CLOSEOUT] note=${noteResult.status === 0 ? "PASS" : "FAIL"}`);
  const renderedNoteOutput = ensureTrailingNewline(`${noteResult.stdout || ""}${noteResult.stderr || ""}`.trimEnd());
  for (const line of renderedNoteOutput.trimEnd().split("\n")) {
    if (line.trim()) outputLines.push(`  ${line}`);
  }

  let syncResultOk = true;
  if (syncOptions?.modeSpec) {
    const syncResult = spawnSync(process.execPath, [
      workflowDossierScript,
      "sync",
      normalizedWpId,
      "--role",
      "INTEGRATION_VALIDATOR",
      "--tag",
      "CLOSEOUT_SYNC",
      "--surface",
      "PHASE_CHECK_CLOSEOUT",
      "--file",
      existingDossierPath,
    ], {
      cwd: phaseCheckCwd,
      encoding: "utf8",
      env: process.env,
    });
    syncResultOk = syncResult.status === 0;
    outputLines.push(`[WORKFLOW_DOSSIER_CLOSEOUT] sync=${syncResultOk ? "PASS" : "FAIL"}`);
    const renderedSyncOutput = ensureTrailingNewline(`${syncResult.stdout || ""}${syncResult.stderr || ""}`.trimEnd());
    for (const line of renderedSyncOutput.trimEnd().split("\n")) {
      if (line.trim()) outputLines.push(`  ${line}`);
    }
  } else {
    outputLines.push("[WORKFLOW_DOSSIER_CLOSEOUT] sync=SKIP (no terminal closeout sync mode requested)");
  }

  const injectResult = spawnSync(process.execPath, [
    workflowDossierScript,
    "inject-repomem",
    normalizedWpId,
    "--file",
    existingDossierPath,
  ], {
    cwd: phaseCheckCwd,
    encoding: "utf8",
    env: process.env,
  });
  const injectResultOk = injectResult.status === 0;
  outputLines.push(`[WORKFLOW_DOSSIER_CLOSEOUT] repomem_import=${injectResultOk ? "PASS" : "FAIL"}`);
  const renderedInjectOutput = ensureTrailingNewline(`${injectResult.stdout || ""}${injectResult.stderr || ""}`.trimEnd());
  for (const line of renderedInjectOutput.trimEnd().split("\n")) {
    if (line.trim()) outputLines.push(`  ${line}`);
  }

  // Append wp-metrics summary to the dossier at closeout (direct import, no subprocess).
  try {
    const metricsArtifacts = loadWpTimelineArtifacts(REPO_ROOT, normalizedWpId);
    const metricsEntries = buildWpTimelineEntries({
      threadEntries: metricsArtifacts.threadEntries,
      receipts: metricsArtifacts.receipts,
      notifications: metricsArtifacts.notifications,
      controlRequests: metricsArtifacts.controlRequests,
      controlResults: metricsArtifacts.controlResults,
      tokenCommands: metricsArtifacts.tokenLedger?.commands || [],
    });
    const metricsSpans = buildWpTimelineSpans({
      receipts: metricsArtifacts.receipts,
      controlRequests: metricsArtifacts.controlRequests,
      controlResults: metricsArtifacts.controlResults,
      tokenCommands: metricsArtifacts.tokenLedger?.commands || [],
    });
    const metricsSummary = buildWpTimelineSummary({
      wpId: normalizedWpId,
      packetPath: metricsArtifacts.packetPath,
      workflowLane: metricsArtifacts.workflowLane,
      runtimeStatus: metricsArtifacts.runtimeStatus,
      receipts: metricsArtifacts.receipts,
      notifications: metricsArtifacts.notifications,
      controlRequests: metricsArtifacts.controlRequests,
      controlResults: metricsArtifacts.controlResults,
      tokenLedger: metricsArtifacts.tokenLedger,
      entries: metricsEntries,
      spans: metricsSpans,
    });
    const m = buildWpMetrics({
      wpId: normalizedWpId,
      summary: metricsSummary,
      receipts: metricsArtifacts.receipts,
      controlResults: metricsArtifacts.controlResults,
    });
    const metricsLine = [
      `wall_clock=${m.wall_clock_minutes ?? "?"}min`,
      `active=${m.product_active_minutes ?? "?"}min`,
      `repair=${m.repair_minutes ?? "?"}min`,
      `validator_wait=${m.validator_wait_minutes ?? "?"}min`,
      `route_wait=${m.route_wait_minutes ?? "?"}min`,
      `gov_overhead=${m.governance_overhead_ratio ?? "?"}`,
      `receipts=${m.receipt_count ?? "?"}`,
      `dup_receipts=${m.duplicate_receipts ?? "?"}`,
      `stale_routes=${m.stale_route_incidents ?? "?"}`,
      `acp_cmds=${m.acp_commands ?? "?"}`,
      `acp_fail=${m.acp_failures ?? "?"}`,
      `restarts=${m.session_restarts ?? "?"}`,
      `mt=${m.mt_count ?? "?"}`,
      `fix_cycles=${m.fix_cycles ?? "?"}`,
      `zero_exec=${m.zero_execution_incidents ?? "?"}`,
      `tokens_in=${m.token_input_total ?? "?"}`,
      `tokens_out=${m.token_output_total ?? "?"}`,
      `turns=${m.token_turn_count ?? "?"}`,
    ].join(" | ");
    const workflowDossierScript = path.resolve(GOV_ROOT_ABS, "roles_shared", "scripts", "audit", "workflow-dossier.mjs");
    spawnSync(process.execPath, [
      workflowDossierScript, "note", normalizedWpId, "EXECUTION", metricsLine,
      "--role", "INTEGRATION_VALIDATOR", "--tag", "METRICS", "--surface", "wp-metrics",
      "--file", existingDossierPath,
    ], { cwd: phaseCheckCwd, encoding: "utf8", env: process.env });
    outputLines.push(`[WORKFLOW_DOSSIER_CLOSEOUT] metrics=APPENDED`);
  } catch {
    outputLines.push(`[WORKFLOW_DOSSIER_CLOSEOUT] metrics=SKIP (extraction error)`);
  }

  return {
    ok: noteResult.status === 0 && syncResultOk && injectResultOk,
    output: `${outputLines.join("\n")}\n`,
  };
}

function readPacketText(wpId) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) return "";
  const resolved = resolveWorkPacketPath(normalizedWpId);
  const wpPath = resolved?.packetPath
    ? repoPathAbs(resolved.packetPath)
    : repoPathAbs(`${GOV_ROOT_REPO_REL}/task_packets/${normalizedWpId}.md`);
  try {
    return fs.readFileSync(wpPath, "utf8");
  } catch {
    return "";
  }
}

function runStep(step) {
  const { label, args = [] } = step;
  if (label === "gate-check") {
    const result = runGateCheck(args[0]);
    return {
      ok: result.ok,
      output: ensureTrailingNewline(result.output.trimEnd()),
    };
  }
  if (label === "ensure-wp-communications") {
    const result = ensureWpCommunications({ wpId: args[0] });
    return {
      ok: true,
      output: [
        `[WP_COMMUNICATIONS] ready ${result.dir}`,
        `- THREAD.md: ${result.threadFile}`,
        `- RUNTIME_STATUS.json: ${result.runtimeStatusFile}`,
        `- RECEIPTS.jsonl: ${result.receiptsFile}`,
        "",
      ].join("\n"),
    };
  }
  if (label === "active-lane-brief") {
    const brief = buildActiveLaneBrief({
      role: args[0],
      wpId: args[1],
    });
    return {
      ok: true,
      output: formatActiveLaneBrief(brief),
    };
  }
  if (label === "wp-communication-health-check") {
    const result = buildWpCommunicationHealthCheckResult({
      wpId: args[0],
      stage: args[1],
      actorRole: args[2],
      actorSession: args[3],
    });
    return {
      ok: result.ok,
      output: formatWpCommunicationHealthCheckResult(result),
    };
  }
  if (label === "integration-validator-context-brief") {
    const validatorCwd = resolveCloseoutSyncCwd({
      wpId: args[0],
      phaseCheckCwd: resolvePhaseCheckCwd(),
    });
    const brief = buildIntegrationValidatorContextBriefFromEnvironment({
      wpId: args[0],
      repoRoot: validatorCwd,
      gitContext: currentGitContextAt(validatorCwd),
    });
    return {
      ok: true,
      output: formatIntegrationValidatorContextBrief(brief),
    };
  }
  if (label === "validator-handoff-check") {
    const committedTargetArgs = parseCommittedTargetArgs(args.slice(1));
    const result = buildValidatorHandoffCheckResult({
      wpId: args[0],
      rev: committedTargetArgs.rev,
      range: committedTargetArgs.range,
    });
    return {
      ok: result.ok,
      output: formatValidatorHandoffCheckResult(result),
    };
  }
  if (label === "validator-packet-complete") {
    const result = buildValidatorPacketCompleteResult({
      wpId: args[0],
    });
    return {
      ok: result.ok,
      output: formatValidatorPacketCompleteResult(result),
    };
  }
  if (label === "integration-validator-closeout-check") {
    const validatorCwd = resolveCloseoutSyncCwd({
      wpId: args[0],
      phaseCheckCwd: resolvePhaseCheckCwd(),
    });
    const result = buildIntegrationValidatorCloseoutCheckResult({
      wpId: args[0],
      allowSyncRepair: args.slice(1).includes("--sync-mode"),
      repoRootOverride: validatorCwd,
      gitContextOverride: currentGitContextAt(validatorCwd),
    });
    return {
      ok: result.ok,
      output: formatIntegrationValidatorCloseoutCheckResult(result),
      resultData: result,
    };
  }
  return runSubprocessStep(step);
}

const ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);

function parseCliArgs(argv = process.argv.slice(2)) {
  const [phaseArg = "", wpIdArg = "", ...restArgs] = argv;
  const tokens = restArgs.filter((value) => value !== "");
  let roleArg = "";
  let sessionArg = "";
  const extraArgs = [];

  if (tokens.length > 0 && !tokens[0].startsWith("--") && ROLE_VALUES.has(String(tokens[0] || "").trim().toUpperCase())) {
    roleArg = tokens.shift();
  }
  if (tokens.length > 0 && !tokens[0].startsWith("--")) {
    sessionArg = tokens.shift();
  }
  extraArgs.push(...tokens);

  return {
    phaseArg,
    wpIdArg,
    roleArg,
    sessionArg,
    extraArgs,
    verbose: extraArgs.includes("--verbose"),
  };
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function workflowLaneForPacket(wpId) {
  try {
    const packetText = readPacketText(wpId);
    return parseSingleField(packetText, "WORKFLOW_LANE").toUpperCase();
  } catch {
    return "";
  }
}

function escapeRegex(value) {
  return String(value || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function hasCommitBySubjectRegex(subjectRegex) {
  const result = spawnSync("git", ["log", "-n", "1", "--format=%H", `--grep=${subjectRegex}`], {
    encoding: "utf8",
  });
  if (result.status !== 0) return false;
  return Boolean(String(result.stdout || "").trim());
}

function resolveCloseoutPublicationTruth({
  wpId = "",
  packetText = "",
  repoRoot = resolvePhaseCheckCwd(),
  runtimeStatusOverride = undefined,
} = {}) {
  const parsedTruth = parseMergeProgressionTruth(packetText);
  const declaredRuntime = loadDeclaredRuntimeStatus({
    repoRoot,
    packetContent: packetText,
    runtimeStatusOverride,
  });
  const publication = readExecutionPublicationView({
    runtimeStatus: declaredRuntime.runtimeStatus,
    packetStatus: parsedTruth.status,
  });

  return {
    parsedTruth,
    publication,
    effectivePacketStatus: publication.packet_status || parsedTruth.status,
    effectiveTaskBoardStatus: publication.task_board_status || "",
    effectiveMainContainmentStatus:
      String(publication.runtime?.main_containment_status || "").trim().toUpperCase()
      || parsedTruth.mainContainmentStatus,
    effectiveMergedMainCommit:
      String(publication.runtime?.merged_main_commit || "").trim()
      || parsedTruth.mergedMainCommit,
    effectiveValidationVerdict: inferValidationVerdictFromPublication({
      packetStatus: publication.packet_status || parsedTruth.status,
      taskBoardStatus: publication.task_board_status || "",
      fallbackVerdict: parsedTruth.validationVerdict,
    }),
  };
}

async function printFailLog(wpId) {
  try {
    const { DatabaseSync } = await import("node:sqlite");
    const memDbPath = path.join(repoPathAbs(".."), "gov_runtime", "roles_shared", "GOVERNANCE_MEMORY.db");
    if (!fs.existsSync(memDbPath)) return;
    const db = new DatabaseSync(memDbPath, { readOnly: true });
    try {
      const patterns = db.prepare(
        `SELECT topic, summary FROM memory_index
         WHERE memory_type = 'procedural' AND consolidated = 0
           AND (wp_id = ? OR wp_id = '')
         ORDER BY importance DESC LIMIT 5`,
      ).all(wpId);
      if (patterns.length === 0) return;
      console.log("");
      console.log("FAIL_LOG [CX-503K]");
      for (const pattern of patterns) {
        console.log(`- ${pattern.topic}: ${String(pattern.summary || "").slice(0, 120)}`);
      }
    } finally {
      try { db.close(); } catch {}
    }
  } catch {
    // best-effort only
  }
}

// [CX-109D] Forbidden worktree directories for coder sessions.
const CODER_FORBIDDEN_WORKTREE_NAMES = ["handshake_main", "wt-gov-kernel", "wt-ilja"];

function checkCoderWorktreeConfinement(wpId) {
  const cwd = path.resolve(process.cwd());
  const cwdBase = path.basename(cwd);
  const forbidden = CODER_FORBIDDEN_WORKTREE_NAMES.find((name) => cwdBase === name);
  if (forbidden) {
    return { ok: false, why: `CODER_WORKTREE_BREACH [CX-109D]: coder session is running in forbidden directory '${forbidden}'. Coder must operate in the declared WP worktree.` };
  }
  const expectedWorktreeDir = defaultCoderWorktreeDir(wpId);
  const expectedBase = path.basename(path.resolve(REPO_ROOT, expectedWorktreeDir));
  if (cwdBase !== expectedBase) {
    return { ok: false, why: `CODER_WORKTREE_MISMATCH [CX-109D]: coder cwd '${cwdBase}' does not match declared WP worktree '${expectedBase}'. Coder must operate in the assigned WP worktree.` };
  }
  return { ok: true, why: "" };
}

function buildStartupCoderOutcome({
  wpId,
  session = "",
  args = [],
  verbose = false,
  stepResults = new Map(),
  defaultOk,
  defaultWhy,
}) {
  let ok = defaultOk;
  let why = defaultWhy;

  // [CX-109D] Worktree confinement check — must run before any other startup logic.
  const confinement = checkCoderWorktreeConfinement(wpId);
  if (!confinement.ok) {
    ok = false;
    why = confinement.why;
  }

  const preWorkResult = stepResults.get("pre-work-check");
  const blockedOnBootstrapClaim = /Missing docs-only bootstrap claim commit/i.test(String(preWorkResult?.output || ""));
  const workflowLane = workflowLaneForPacket(wpId);
  const usesSkeletonCheckpointGate = workflowLane !== "ORCHESTRATOR_MANAGED";
  const skeletonApprover =
    workflowLane === "ORCHESTRATOR_MANAGED" ? "Orchestrator/Validator/Operator" : "Operator/Validator";
  const checkpointSubjectRe = `^docs: skeleton checkpoint \\[${escapeRegex(wpId)}\\]$`;
  const approvedSubjectRe = `^docs: skeleton approved \\[${escapeRegex(wpId)}\\]$`;
  const hasSkeletonCheckpoint = hasCommitBySubjectRegex(checkpointSubjectRe);
  const hasSkeletonApproval = hasCommitBySubjectRegex(approvedSubjectRe);
  let blockedOnSkeletonApproval = false;

  if (usesSkeletonCheckpointGate && ok && hasSkeletonCheckpoint && !hasSkeletonApproval) {
    ok = false;
    blockedOnSkeletonApproval = true;
    why = `Skeleton checkpoint exists; awaiting ${skeletonApprover} approval.`;
  }

  const rerunArgs = verbose ? args : [...args.filter((value) => value !== "--verbose"), "--verbose"];
  const rerunVerbose = buildPhaseCheckCommand({
    phase: "STARTUP",
    wpId,
    role: "CODER",
    session,
    args: rerunArgs,
  });
  const rerunDefault = buildPhaseCheckCommand({
    phase: "STARTUP",
    wpId,
    role: "CODER",
    session,
    args: args.filter((value) => value !== "--verbose"),
  });

  const nextCommands = [];
  if (ok) {
    if (!verbose) nextCommands.push(`For full nested gate output: ${rerunVerbose}`);
    if (!usesSkeletonCheckpointGate) {
      nextCommands.push("Proceed to implementation.");
    } else if (!hasSkeletonCheckpoint) {
      nextCommands.push(`(After updating the packet \`## SKELETON\`) just coder-skeleton-checkpoint ${wpId}`);
      nextCommands.push(`STOP: Await skeleton approval (${skeletonApprover} runs: just skeleton-approved ${wpId})`);
      nextCommands.push(`After approval commit exists: re-run ${rerunDefault}`);
    } else {
      nextCommands.push("Proceed to implementation (skeleton approved).");
    }
  } else if (blockedOnSkeletonApproval) {
    if (!verbose) nextCommands.push(`For full nested gate output: ${rerunVerbose}`);
    nextCommands.push(`STOP: Await skeleton approval (${skeletonApprover} runs: just skeleton-approved ${wpId})`);
    nextCommands.push(`After approval commit exists: re-run ${rerunDefault}`);
  } else if (blockedOnBootstrapClaim) {
    if (!verbose) nextCommands.push(`For full nested gate output: ${rerunVerbose}`);
    nextCommands.push(`Create the required docs-only bootstrap claim commit: node .GOV/roles/coder/checks/coder-bootstrap-claim.mjs ${wpId}`);
    nextCommands.push(`Preserve the checkpoint remotely: just backup-push feat/${wpId} feat/${wpId}`);
    nextCommands.push(`Re-run: ${rerunDefault}`);
  } else {
    if (!verbose) nextCommands.push(`For full nested gate output: ${rerunVerbose}`);
    nextCommands.push("Review the failures above.");
    nextCommands.push(`Fix the packet/worktree context, then re-run: ${rerunDefault}`);
  }

  return {
    ok,
    why,
    nextCommands,
  };
}

export function buildCloseoutNextCommands({
  wpId,
  args = [],
  verbose = false,
  ok = true,
  syncOptions = {},
  workflowDossierCloseoutOk = true,
  runtimeStatusOverride = undefined,
  integrationValidatorCloseoutResult = null,
} = {}) {
  const rerunArgs = verbose ? args : [...args.filter((value) => value !== "--verbose"), "--verbose"];
  const rerunVerbose = buildPhaseCheckCommand({
    phase: "CLOSEOUT",
    wpId,
    args: rerunArgs,
  });
  const rerunDefault = buildPhaseCheckCommand({
    phase: "CLOSEOUT",
    wpId,
    args: args.filter((value) => value !== "--verbose"),
  });
  const nextCommands = [];
  const latestCloseoutGovernedAction = integrationValidatorCloseoutResult?.closeoutSyncGovernance?.latestGovernedAction || null;
  const latestCloseoutEvent = integrationValidatorCloseoutResult?.closeoutSyncGovernance?.latestEvent || null;
  const closeoutDependencyView = integrationValidatorCloseoutResult?.closeoutDependencyView || null;
  const governedCloseoutMarker = latestCloseoutGovernedAction
    ? `${latestCloseoutGovernedAction.rule_id || "NONE"} @ ${latestCloseoutGovernedAction.updated_at || "<missing>"}`
    : "";
  if (!verbose) nextCommands.push(`For full nested gate output: ${rerunVerbose}`);
  if (!ok) {
    nextCommands.push("Review the failures above.");
    if (closeoutDependencyView?.summary) {
      nextCommands.push(`Canonical closeout dependency view: ${closeoutDependencyView.summary}`);
    }
    nextCommands.push(`Fix the closeout topology or governed truth issue, then re-run: ${rerunDefault}`);
    return nextCommands;
  }

  if (!workflowDossierCloseoutOk) {
    nextCommands.push(`Repair the Workflow Dossier closeout import path: just workflow-dossier-inject-repomem ${wpId}`);
    nextCommands.push(`If mechanical telemetry is missing, also run: just workflow-dossier-sync ${wpId} --role INTEGRATION_VALIDATOR --tag CLOSEOUT_SYNC --surface PHASE_CHECK_CLOSEOUT`);
  }

  const syncMode = String(syncOptions?.modeSpec?.mode || "").trim().toUpperCase();
  if (syncMode === "MERGE_PENDING") {
    nextCommands.push(
      latestCloseoutGovernedAction
        ? `Mechanical Workflow Dossier closeout sync and repomem import are recorded via governed action ${governedCloseoutMarker}. Add the final review/rubric only after terminal closeout.`
        : "Mechanical Workflow Dossier closeout sync and repomem import are recorded. Add the final review/rubric only after terminal closeout.",
    );
    nextCommands.push(
      `After local main containment is real: just phase-check CLOSEOUT ${wpId} --sync-mode CONTAINED_IN_MAIN --merged-main-sha <MERGED_MAIN_SHA> --context "<why contained-main closure is now valid, >=40 chars>"`,
    );
    return nextCommands;
  }
  if (syncMode === "CONTAINED_IN_MAIN") {
    nextCommands.push(
      latestCloseoutGovernedAction
        ? `Append the final Workflow Dossier post-mortem/review and fill the closeout rubric in the active dossier. The mechanical closeout sync and repomem import are already appended via governed action ${governedCloseoutMarker}.`
        : "Append the final Workflow Dossier post-mortem/review and fill the closeout rubric in the active dossier. The mechanical closeout sync and repomem import are already appended.",
    );
    nextCommands.push(`Proceed to final PASS gate flow (for example: just validator-gate-commit ${wpId}).`);
    return nextCommands;
  }
  if (syncMode) {
    nextCommands.push(
      latestCloseoutGovernedAction
        ? `Append the final Workflow Dossier post-mortem/review and fill the closeout rubric in the active dossier. The mechanical closeout sync and repomem import are already appended via governed action ${governedCloseoutMarker}.`
        : "Append the final Workflow Dossier post-mortem/review and fill the closeout rubric in the active dossier. The mechanical closeout sync and repomem import are already appended.",
    );
    nextCommands.push("Proceed with the remaining validator gate flow for the recorded terminal verdict.");
    return nextCommands;
  }

  const packetText = readPacketText(wpId);
  const closeoutTruth = resolveCloseoutPublicationTruth({
    wpId,
    packetText,
    repoRoot: resolvePhaseCheckCwd(),
    runtimeStatusOverride,
  });
  const effectiveMainContainmentStatus = closeoutDependencyView?.publication?.main_containment_status || closeoutTruth.effectiveMainContainmentStatus;
  const effectiveValidationVerdict = closeoutDependencyView?.publication?.validation_verdict || closeoutTruth.effectiveValidationVerdict;
  let suggestedMode = "";
  let suggestedMergedMainSha = false;
  if (effectiveValidationVerdict === "PASS") {
    if (effectiveMainContainmentStatus === "CONTAINED_IN_MAIN" || effectiveMainContainmentStatus === "NOT_REQUIRED") {
      nextCommands.push(
        latestCloseoutEvent?.mode === "CONTAINED_IN_MAIN" && latestCloseoutGovernedAction
          ? `Closeout truth already reflects contained-main PASS via governed action ${governedCloseoutMarker}. Proceed to the remaining final gate flow (for example: just validator-gate-commit ${wpId}).`
          : `Closeout truth already reflects contained-main PASS. Proceed to the remaining final gate flow (for example: just validator-gate-commit ${wpId}).`,
      );
      return nextCommands;
    }
    if (effectiveMainContainmentStatus === "MERGE_PENDING") {
      if (latestCloseoutEvent?.mode === "MERGE_PENDING" && latestCloseoutGovernedAction) {
        nextCommands.push(
          `Merge-pending closeout sync is already recorded via governed action ${governedCloseoutMarker}; promote to contained-main only after the local main merge is real.`,
        );
      }
      suggestedMode = "CONTAINED_IN_MAIN";
      suggestedMergedMainSha = true;
    } else {
      suggestedMode = "MERGE_PENDING";
    }
  } else if (effectiveValidationVerdict === "FAIL") {
    suggestedMode = "FAIL";
  } else if (effectiveValidationVerdict === "OUTDATED_ONLY") {
    suggestedMode = "OUTDATED_ONLY";
  } else if (effectiveValidationVerdict === "ABANDONED") {
    suggestedMode = "ABANDONED";
  }

  if (!suggestedMode) {
    nextCommands.push("No additional closeout sync suggestion could be derived from the current packet truth.");
    return nextCommands;
  }
  nextCommands.push(
    `Run governed closeout truth sync through this same phase command: just phase-check CLOSEOUT ${wpId} --sync-mode ${suggestedMode}${suggestedMergedMainSha ? " --merged-main-sha <MERGED_MAIN_SHA>" : ""} --context "<why this closeout truth is being recorded, >=40 chars>"`,
  );
  return nextCommands;
}

function printStepSummary({ label, result, verbose = false }) {
  console.log(`- ${label}: ${result.ok ? "PASS" : "FAIL"}`);
  if (verbose) {
    const renderedOutput = ensureTrailingNewline(result.output.trimEnd());
    for (const line of renderedOutput.trimEnd().split("\n")) {
      console.log(`  ${line}`);
    }
    return;
  }
  for (const line of compactGateOutputSummary(result.output)) {
    console.log(`  ${line}`);
  }
}

async function runCli() {
  const {
    phaseArg,
    wpIdArg,
    roleArg,
    sessionArg,
    extraArgs,
    verbose,
  } = parseCliArgs();
  if (!phaseArg || !wpIdArg) {
    printUsage();
    process.exit(1);
  }

  try {
    const normalizedPhase = String(phaseArg || "").trim().toUpperCase();
    const normalizedRole = String(roleArg || "").trim().toUpperCase();
    if (normalizedPhase !== "CLOSEOUT" && hasCloseoutSyncFlags(extraArgs)) {
      throw new Error("CLOSEOUT sync flags are only valid for phase-check CLOSEOUT");
    }
    const closeoutSyncOptions = normalizedPhase === "CLOSEOUT"
      ? parseCloseoutSyncOptions(extraArgs)
      : {
        modeSpec: null,
        context: "",
        mergedMainSha: "",
        debug: false,
      };
    const plan = buildPhaseCheckPlan({
      phase: phaseArg,
      wpId: wpIdArg,
      role: roleArg,
      session: sessionArg,
      args: extraArgs,
    });
    const sections = [];
    const stepResults = new Map();
    let ok = true;
    let why = `${normalizedPhase} phase checks passed.`;
    let deferredCloseoutMaintenanceStep = null;
    let workflowDossierCloseoutResult = null;

    console.log(`PHASE_CHECK_OUTPUT [CX-PHASE-CHECK-001]`);
    console.log("");
    for (const currentStep of plan) {
      if (
        normalizedPhase === "CLOSEOUT"
        && closeoutSyncOptions.modeSpec
        && currentStep.label === "launch-memory-manager"
      ) {
        deferredCloseoutMaintenanceStep = currentStep;
        continue;
      }
      const result = runStep(currentStep);
      stepResults.set(currentStep.label, result);
      sections.push({ title: currentStep.label, body: result.output });
      printStepSummary({ label: currentStep.label, result, verbose });
      if (!result.ok && ok) {
        ok = false;
        why = `${currentStep.label} failed.`;
      }
    }

    if (normalizedPhase === "CLOSEOUT" && closeoutSyncOptions.modeSpec && ok) {
      const syncResult = runCloseoutSyncStep({
        wpId: wpIdArg,
        syncOptions: closeoutSyncOptions,
      });
      stepResults.set("closeout-truth-sync", syncResult);
      sections.push({ title: "closeout-truth-sync", body: syncResult.output });
      printStepSummary({ label: "closeout-truth-sync", result: syncResult, verbose });
      if (!syncResult.ok) {
        ok = false;
        why = "closeout-truth-sync failed.";
      }
    }

    if (deferredCloseoutMaintenanceStep && ok) {
      const maintenanceResult = runStep(deferredCloseoutMaintenanceStep);
      stepResults.set(deferredCloseoutMaintenanceStep.label, maintenanceResult);
      sections.push({ title: deferredCloseoutMaintenanceStep.label, body: maintenanceResult.output });
      printStepSummary({ label: deferredCloseoutMaintenanceStep.label, result: maintenanceResult, verbose });
      if (!maintenanceResult.ok) {
        ok = false;
        why = `${deferredCloseoutMaintenanceStep.label} failed.`;
      }
    }

    if (normalizedPhase === "CLOSEOUT" && ok) {
      const terminalCleanupResult = runTerminalReadySessionCloseStep({
        wpId: wpIdArg,
      });
      stepResults.set("close-terminal-sessions", terminalCleanupResult);
      sections.push({ title: "close-terminal-sessions", body: terminalCleanupResult.output });
      printStepSummary({ label: "close-terminal-sessions", result: terminalCleanupResult, verbose });
      if (!terminalCleanupResult.ok) {
        ok = false;
        why = "close-terminal-sessions failed.";
      }
    }

    if (normalizedPhase === "CLOSEOUT") {
      workflowDossierCloseoutResult = runWorkflowDossierCloseoutStep({
        wpId: wpIdArg,
        ok,
        syncOptions: closeoutSyncOptions,
        why,
      });
      stepResults.set("workflow-dossier-closeout", workflowDossierCloseoutResult);
      sections.push({ title: "workflow-dossier-closeout", body: workflowDossierCloseoutResult.output });
      printStepSummary({ label: "workflow-dossier-closeout", result: workflowDossierCloseoutResult, verbose });
    }

    if (normalizedPhase === "STARTUP" && normalizedRole === "CODER") {
      const startupOutcome = buildStartupCoderOutcome({
        wpId: wpIdArg,
        session: sessionArg,
        args: extraArgs,
        verbose,
        stepResults,
        defaultOk: ok,
        defaultWhy: why,
      });
      ok = startupOutcome.ok;
      why = startupOutcome.why;
      await printFailLog(wpIdArg);
      if (!ok) {
        captureCheckFindings({ check: "phase-check-startup", findings: [why], wpId: wpIdArg });
      }
      console.log("");
      console.log("NEXT_COMMANDS [CX-PHASE-CHECK-001]");
      for (const line of startupOutcome.nextCommands) {
        console.log(`- ${line}`);
      }
    } else if (normalizedPhase === "CLOSEOUT") {
      const closeoutNextCommands = buildCloseoutNextCommands({
        wpId: wpIdArg,
        args: extraArgs,
        verbose,
        ok,
        syncOptions: closeoutSyncOptions,
        workflowDossierCloseoutOk: workflowDossierCloseoutResult?.ok !== false,
        integrationValidatorCloseoutResult: stepResults.get("integration-validator-closeout-check")?.resultData || null,
      });
      console.log("");
      console.log("NEXT_COMMANDS [CX-PHASE-CHECK-001]");
      for (const line of closeoutNextCommands) {
        console.log(`- ${line}`);
      }
    }

    const artifactPath = writeGateOutputArtifact(`phase-check-${normalizedPhase.toLowerCase()}`, wpIdArg, sections);
    console.log("");
    console.log(`PHASE_CHECK_STATUS [CX-PHASE-CHECK-001]`);
    console.log(`- PHASE: ${normalizedPhase}`);
    console.log(`- GATE_RAN: ${buildPhaseCheckCommand({
      phase: normalizedPhase,
      wpId: wpIdArg,
      role: roleArg,
      session: sessionArg,
      args: extraArgs,
    })}`);
    console.log(`- ARTIFACT_PATH: ${artifactPath}`);
    console.log(`- RESULT: ${ok ? "PASS" : "FAIL"}`);
    console.log(`- WHY: ${why}`);
    process.exit(ok ? 0 : 1);
  } catch (error) {
    printUsage(error?.message || String(error || ""));
    process.exit(1);
  }
}

const currentFilePath = path.resolve(fileURLToPath(import.meta.url));
const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (currentFilePath === invokedPath) {
  runCli();
}
