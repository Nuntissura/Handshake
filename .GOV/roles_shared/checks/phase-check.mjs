#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { GOV_ROOT_REPO_REL, REPO_ROOT, repoPathAbs, resolveWorkPacketPath } from "../scripts/lib/runtime-paths.mjs";
import { compactGateOutputSummary, writeGateOutputArtifact } from "../scripts/lib/gate-output-artifact-lib.mjs";
import { ensureWpCommunications } from "../scripts/wp/ensure-wp-communications.mjs";
import { captureCheckFindings } from "../scripts/memory/memory-capture-from-check.mjs";
import { buildActiveLaneBrief, formatActiveLaneBrief } from "../scripts/session/active-lane-brief-lib.mjs";
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
} from "../../roles/validator/scripts/lib/integration-validator-closeout-lib.mjs";
import {
  buildValidatorPacketCompleteResult,
  formatValidatorPacketCompleteResult,
  buildValidatorHandoffCheckResult,
  formatValidatorHandoffCheckResult,
} from "../../roles/validator/scripts/lib/validator-governance-lib.mjs";
import { parseMergeProgressionTruth } from "../scripts/lib/merge-progression-truth-lib.mjs";

export { buildPhaseCheckCommand, buildPhaseCheckPlan, PHASE_VALUES } from "./phase-check-lib.mjs";

const CLOSEOUT_SYNC_SHA_RE = /^[0-9a-f]{7,40}$/i;
const CLOSEOUT_SYNC_FLAG_SET = new Set(["--sync-mode", "--context", "--merged-main-sha", "--sync-debug"]);

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
  const wpPath = resolved?.absolutePath || repoPathAbs(`${GOV_ROOT_REPO_REL}/task_packets/${normalizedWpId}.md`);
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
  const mode = String(rawMode || "").trim().toUpperCase();
  if (mode === "MERGE_PENDING" || mode === "DONE_MERGE_PENDING") {
    return {
      mode: "MERGE_PENDING",
      requireMergedMainCommit: false,
    };
  }
  if (mode === "CONTAINED_IN_MAIN" || mode === "DONE_VALIDATED") {
    return {
      mode: "CONTAINED_IN_MAIN",
      requireMergedMainCommit: true,
    };
  }
  if (mode === "FAIL" || mode === "DONE_FAIL") {
    return {
      mode: "FAIL",
      requireMergedMainCommit: false,
    };
  }
  if (mode === "OUTDATED_ONLY" || mode === "DONE_OUTDATED_ONLY") {
    return {
      mode: "OUTDATED_ONLY",
      requireMergedMainCommit: false,
    };
  }
  if (mode === "ABANDONED" || mode === "DONE_ABANDONED") {
    return {
      mode: "ABANDONED",
      requireMergedMainCommit: false,
    };
  }
  return null;
}

function hasCloseoutSyncFlags(args = []) {
  return (args || []).some((value) => CLOSEOUT_SYNC_FLAG_SET.has(String(value || "").trim()));
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
      const nextValue = String(tokens[index + 1] || "");
      if (!nextValue || String(nextValue).trim().startsWith("--")) {
        throw new Error(`${token} requires a value`);
      }
      if (token === "--sync-mode") rawMode = nextValue;
      if (token === "--context") context = nextValue;
      if (token === "--merged-main-sha") mergedMainSha = nextValue;
      index += 1;
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
  const outputChunks = [];
  const repomemGateResult = spawnSync(process.execPath, [repomemScript, "gate"], {
    cwd: REPO_ROOT,
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
    cwd: REPO_ROOT,
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
    cwd: REPO_ROOT,
    encoding: "utf8",
  });
  outputChunks.push(ensureTrailingNewline(`${result.stdout || ""}${result.stderr || ""}`.trimEnd()));
  return {
    ok: result.status === 0,
    output: outputChunks.join(""),
  };
}

function readPacketText(wpId) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) return "";
  const resolved = resolveWorkPacketPath(normalizedWpId);
  const wpPath = resolved?.absolutePath || repoPathAbs(`${GOV_ROOT_REPO_REL}/task_packets/${normalizedWpId}.md`);
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
    const brief = buildIntegrationValidatorContextBriefFromEnvironment({
      wpId: args[0],
    });
    return {
      ok: true,
      output: formatIntegrationValidatorContextBrief(brief),
    };
  }
  if (label === "validator-handoff-check") {
    const result = buildValidatorHandoffCheckResult({
      wpId: args[0],
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
    const result = buildIntegrationValidatorCloseoutCheckResult({
      wpId: args[0],
    });
    return {
      ok: result.ok,
      output: formatIntegrationValidatorCloseoutCheckResult(result),
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

function buildCloseoutNextCommands({
  wpId,
  args = [],
  verbose = false,
  ok = true,
  syncOptions = {},
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
  if (!verbose) nextCommands.push(`For full nested gate output: ${rerunVerbose}`);
  if (!ok) {
    nextCommands.push("Review the failures above.");
    nextCommands.push(`Fix the closeout topology or governed truth issue, then re-run: ${rerunDefault}`);
    return nextCommands;
  }

  const syncMode = String(syncOptions?.modeSpec?.mode || "").trim().toUpperCase();
  if (syncMode === "MERGE_PENDING") {
    nextCommands.push(
      `After local main containment is real: just phase-check CLOSEOUT ${wpId} --sync-mode CONTAINED_IN_MAIN --merged-main-sha <MERGED_MAIN_SHA> --context "<why contained-main closure is now valid, >=40 chars>"`,
    );
    return nextCommands;
  }
  if (syncMode === "CONTAINED_IN_MAIN") {
    nextCommands.push(`Proceed to final PASS gate flow (for example: just validator-gate-commit ${wpId}).`);
    return nextCommands;
  }
  if (syncMode) {
    nextCommands.push("Proceed with the remaining validator gate flow for the recorded terminal verdict.");
    return nextCommands;
  }

  const packetText = readPacketText(wpId);
  const parsedTruth = parseMergeProgressionTruth(packetText);
  let suggestedMode = "";
  let suggestedMergedMainSha = false;
  if (parsedTruth.validationVerdict === "PASS") {
    if (parsedTruth.mainContainmentStatus === "CONTAINED_IN_MAIN" || /^Validated\s*\(\s*PASS\s*\)$/i.test(parsedTruth.status)) {
      nextCommands.push(`Closeout truth already reflects contained-main PASS. Proceed to the remaining final gate flow (for example: just validator-gate-commit ${wpId}).`);
      return nextCommands;
    }
    if (parsedTruth.mainContainmentStatus === "MERGE_PENDING") {
      suggestedMode = "CONTAINED_IN_MAIN";
      suggestedMergedMainSha = true;
    } else {
      suggestedMode = "MERGE_PENDING";
    }
  } else if (parsedTruth.validationVerdict === "FAIL") {
    suggestedMode = "FAIL";
  } else if (parsedTruth.validationVerdict === "OUTDATED_ONLY") {
    suggestedMode = "OUTDATED_ONLY";
  } else if (parsedTruth.validationVerdict === "ABANDONED") {
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
