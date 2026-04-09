#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  refinementAbsPath,
  resolveRefinementPath,
  resolveWorkPacketPath,
  taskBoardPathAtRepo,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  currentGitContext,
  loadPacket,
  parseClaimField,
  parseCurrentWpStatus,
  parseStatus,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { validateRefinementFile } from "../../../roles_shared/checks/refinement-check.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
registerFailCaptureHook("activation-manager.mjs", { role: "ACTIVATION_MANAGER" });

const SCRIPT_REPO_REL = `${GOV_ROOT_REPO_REL}/roles/activation_manager/scripts/activation-manager.mjs`;
const ACTOR_ROLE = "ACTIVATION_MANAGER";
const DEFAULT_RUNTIME_DIR = path.join(
  GOVERNANCE_RUNTIME_ROOT_ABS,
  "roles",
  "activation_manager",
  "runtime",
  "activation_readiness",
);

function usage() {
  console.error(
    `Usage: node ${SCRIPT_REPO_REL} <startup|prompt|next|readiness> [WP-{ID}] [--write] [--json]`,
  );
  process.exit(1);
}

function fail(message, details = []) {
  failWithMemory("activation-manager.mjs", message, { role: "ACTIVATION_MANAGER", details });
}

function parseArgs(argv) {
  const args = argv.slice(2);
  const action = String(args[0] || "").trim().toLowerCase();
  if (!action) usage();
  const wpId = String(args.find((arg, index) => index > 0 && !String(arg || "").startsWith("--")) || "").trim();
  const flags = new Set(args.filter((arg) => String(arg || "").startsWith("--")));
  return {
    action,
    wpId,
    write: flags.has("--write"),
    json: flags.has("--json"),
  };
}

function runNode(relativeScript, args = []) {
  const scriptPath = path.resolve(REPO_ROOT, relativeScript);
  const result = spawnSync(process.execPath, [scriptPath, ...args], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    ok: result.status === 0,
    stdout: String(result.stdout || "").trim(),
    stderr: String(result.stderr || "").trim(),
    status: result.status ?? 1,
  };
}

function readTaskBoardStatus(wpId) {
  try {
    const taskBoardPath = taskBoardPathAtRepo(REPO_ROOT);
    if (!fs.existsSync(taskBoardPath)) return "";
    const content = fs.readFileSync(taskBoardPath, "utf8");
    const escaped = wpId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = content.match(new RegExp(`^\\|\\s*${escaped}\\s*\\|\\s*([^|]+?)\\s*\\|`, "mi"));
    return match ? match[1].trim() : "";
  } catch {
    return "";
  }
}

function readRefinementText(wpId) {
  const refinementPath = refinementAbsPath(wpId);
  if (!fs.existsSync(refinementPath)) return "";
  try {
    return fs.readFileSync(refinementPath, "utf8");
  } catch {
    return "";
  }
}

function parseMetadataField(text, label) {
  const match = String(text || "").match(new RegExp(`^\\s*-\\s*${label}\\s*:\\s*(.+)\\s*$`, "mi"));
  return match ? match[1].trim() : "";
}

function readPacketContext(wpId) {
  const resolvedPacket = resolveWorkPacketPath(wpId);
  const packetPath = resolvedPacket?.packetPath || "";
  const packetContent = packetPath ? loadPacket(wpId) : "";
  return {
    packetPath,
    packetExists: Boolean(packetPath && packetContent),
    packetContent,
    packetStatus: packetContent ? parseStatus(packetContent) : "",
    currentWpStatus: packetContent ? parseCurrentWpStatus(packetContent) : "",
    workflowLane: packetContent ? parseClaimField(packetContent, "WORKFLOW_LANE") : "",
    executionOwner: packetContent ? parseClaimField(packetContent, "EXECUTION_OWNER") : "",
    userSignature: packetContent ? parseClaimField(packetContent, "USER_SIGNATURE") : "",
    refinementProfile: packetContent ? parseClaimField(packetContent, "REFINEMENT_ENFORCEMENT_PROFILE") : "",
    localBranch: packetContent ? parseClaimField(packetContent, "LOCAL_BRANCH") : "",
    localWorktreeDir: packetContent ? parseClaimField(packetContent, "LOCAL_WORKTREE_DIR") : "",
    remoteBackupBranch: packetContent ? parseClaimField(packetContent, "REMOTE_BACKUP_BRANCH") : "",
    remoteBackupUrl: packetContent ? parseClaimField(packetContent, "REMOTE_BACKUP_URL") : "",
    backupPushStatus: packetContent ? parseClaimField(packetContent, "BACKUP_PUSH_STATUS") : "",
  };
}

function isMissingClaim(value) {
  const normalized = String(value || "").trim();
  return normalized.length === 0 || /^<pending>$/i.test(normalized) || /^<missing>$/i.test(normalized);
}

function countDeclaredMicrotasks(packetPath) {
  if (!packetPath) return 0;
  const packetDir = path.dirname(packetPath);
  if (!fs.existsSync(packetDir)) return 0;
  try {
    return fs.readdirSync(packetDir).filter((entry) => /^MT-\d+\.md$/i.test(entry)).length;
  } catch {
    return 0;
  }
}

function inspectGovKernelLink(localWorktreeDir) {
  if (isMissingClaim(localWorktreeDir)) {
    return {
      status: "NOT_CHECKED",
      worktreeExists: false,
      absoluteWorktreeDir: "",
    };
  }
  const absoluteWorktreeDir = path.resolve(REPO_ROOT, localWorktreeDir);
  if (!fs.existsSync(absoluteWorktreeDir)) {
    return {
      status: "MISSING_WORKTREE",
      worktreeExists: false,
      absoluteWorktreeDir,
    };
  }
  const govDir = path.join(absoluteWorktreeDir, ".GOV");
  if (!fs.existsSync(govDir)) {
    return {
      status: "MISSING_GOV_LINK",
      worktreeExists: true,
      absoluteWorktreeDir,
    };
  }
  try {
    const expected = fs.realpathSync(path.join(REPO_ROOT, ".GOV"));
    const actual = fs.realpathSync(govDir);
    return {
      status: actual === expected ? "KERNEL_LINK_OK" : "WRONG_TARGET",
      worktreeExists: true,
      absoluteWorktreeDir,
    };
  } catch {
    return {
      status: "WRONG_TARGET",
      worktreeExists: true,
      absoluteWorktreeDir,
    };
  }
}

function collectActivationState(wpId) {
  const gitContext = currentGitContext();
  const packet = readPacketContext(wpId);
  const refinementPath = resolveRefinementPath(wpId) || `${GOV_ROOT_REPO_REL}/refinements/${wpId}.md`;
  const refinementAbs = refinementAbsPath(wpId);
  const refinementExists = fs.existsSync(refinementAbs);
  const refinementText = refinementExists ? readRefinementText(wpId) : "";
  const refinementValidation = refinementExists
    ? validateRefinementFile(refinementAbs, { expectedWpId: wpId, requireSignature: false })
    : { ok: false, errors: [`Missing refinement file: ${refinementPath}`] };

  const enrichmentNeeded = parseMetadataField(refinementText, "ENRICHMENT_NEEDED").toUpperCase();
  const clearlyCoversVerdict = parseMetadataField(refinementText, "CLEARLY_COVERS_VERDICT").toUpperCase();
  const userApprovalEvidence = parseMetadataField(refinementText, "USER_APPROVAL_EVIDENCE");
  const refinementSignature = parseMetadataField(refinementText, "USER_SIGNATURE");
  const refinementReviewStatus = parseMetadataField(refinementText, "USER_REVIEW_STATUS").toUpperCase();
  const stubWpIds = parseMetadataField(refinementText, "STUB_WP_IDS");

  const claimCheck = runNode(`${GOV_ROOT_REPO_REL}/roles_shared/checks/task-packet-claim-check.mjs`);
  const traceabilityCheck = runNode(`${GOV_ROOT_REPO_REL}/roles_shared/checks/wp-activation-traceability-check.mjs`);
  const buildOrderCheck = runNode(`${GOV_ROOT_REPO_REL}/roles_shared/checks/build-order-check.mjs`);
  const topologyCheck = packet.packetExists
    ? runNode(`${GOV_ROOT_REPO_REL}/roles_shared/checks/wp-declared-topology-check.mjs`, [wpId])
    : { ok: false, stdout: "", stderr: "", status: 1 };
  const microtaskCount = countDeclaredMicrotasks(packet.packetPath);
  const govKernelLink = inspectGovKernelLink(packet.localWorktreeDir);

  const findings = [];
  const artifactsReady = [];

  if (packet.packetExists) {
    artifactsReady.push(packet.packetPath);
  } else {
    findings.push("packet missing");
  }

  if (refinementExists) {
    artifactsReady.push(refinementPath);
  } else {
    findings.push("refinement missing");
  }

  if (packet.userSignature && !/^<pending>$/i.test(packet.userSignature)) {
    artifactsReady.push("packet USER_SIGNATURE");
  }
  if (refinementSignature && !/^<pending>$/i.test(refinementSignature)) {
    artifactsReady.push("refinement USER_SIGNATURE");
  }
  if (userApprovalEvidence && !/^<pending>$/i.test(userApprovalEvidence)) {
    artifactsReady.push("USER_APPROVAL_EVIDENCE");
  }
  if (!isMissingClaim(packet.localBranch)) {
    artifactsReady.push(`LOCAL_BRANCH=${packet.localBranch}`);
  } else if (packet.packetExists) {
    findings.push("LOCAL_BRANCH missing");
  }
  if (!isMissingClaim(packet.localWorktreeDir) && govKernelLink.worktreeExists) {
    artifactsReady.push(`LOCAL_WORKTREE_DIR=${packet.localWorktreeDir}`);
  } else if (packet.packetExists) {
    findings.push("LOCAL_WORKTREE_DIR missing or unresolved");
  }
  if (!isMissingClaim(packet.remoteBackupBranch)) {
    artifactsReady.push(`REMOTE_BACKUP_BRANCH=${packet.remoteBackupBranch}`);
  } else if (packet.packetExists) {
    findings.push("REMOTE_BACKUP_BRANCH missing");
  }
  if (!isMissingClaim(packet.backupPushStatus)) {
    artifactsReady.push(`BACKUP_PUSH_STATUS=${packet.backupPushStatus}`);
  } else if (packet.packetExists) {
    findings.push("BACKUP_PUSH_STATUS missing");
  }
  if (microtaskCount > 0) {
    artifactsReady.push(`MICROTASKS=${microtaskCount}`);
  }

  if (!refinementValidation.ok) {
    findings.push(...refinementValidation.errors);
  }
  if (!claimCheck.ok) {
    findings.push(`task-packet-claim-check failed: ${claimCheck.stderr || claimCheck.stdout || `exit ${claimCheck.status}`}`);
  }
  if (!traceabilityCheck.ok) {
    findings.push(`wp-activation-traceability-check failed: ${traceabilityCheck.stderr || traceabilityCheck.stdout || `exit ${traceabilityCheck.status}`}`);
  }
  if (!buildOrderCheck.ok) {
    findings.push(`build-order-check failed: ${buildOrderCheck.stderr || buildOrderCheck.stdout || `exit ${buildOrderCheck.status}`}`);
  }
  if (packet.packetExists && !topologyCheck.ok) {
    findings.push(`wp-declared-topology-check failed: ${topologyCheck.stderr || topologyCheck.stdout || `exit ${topologyCheck.status}`}`);
  }
  if (packet.packetExists && govKernelLink.status !== "KERNEL_LINK_OK") {
    findings.push(`gov-kernel-link check failed: ${govKernelLink.status}`);
  }

  let verdict = "READY_FOR_ORCHESTRATOR_REVIEW";
  let nextAction = "Orchestrator review the activation bundle and decide whether to launch downstream lanes.";

  if (!refinementExists || !packet.packetExists) {
    verdict = "REPAIR_REQUIRED";
    nextAction = "Repair or create the missing packet/refinement artifacts before activation can proceed.";
  } else if (enrichmentNeeded === "YES" || clearlyCoversVerdict === "FAIL") {
    verdict = "BLOCKED_BY_SPEC_ENRICHMENT";
    nextAction = "Perform the approved spec-enrichment pass, advance SPEC_CURRENT if needed, then refresh the same WP refinement.";
  } else if (!userApprovalEvidence || /^<pending>$/i.test(userApprovalEvidence) || !refinementSignature || /^<pending>$/i.test(refinementSignature)) {
    verdict = "BLOCKED_BY_OPERATOR_APPROVAL";
    nextAction = `Obtain operator approval evidence and record the one-time signature for ${wpId} before packet activation is treated as ready.`;
  } else if (
    !refinementValidation.ok
    || !claimCheck.ok
    || !traceabilityCheck.ok
    || !buildOrderCheck.ok
    || (packet.packetExists && !topologyCheck.ok)
    || (packet.packetExists && govKernelLink.status !== "KERNEL_LINK_OK")
    || (packet.packetExists && (
      isMissingClaim(packet.localBranch)
      || isMissingClaim(packet.localWorktreeDir)
      || isMissingClaim(packet.remoteBackupBranch)
      || isMissingClaim(packet.backupPushStatus)
    ))
  ) {
    verdict = "REPAIR_REQUIRED";
    nextAction = "Repair refinement, packet, worktree/topology, backup-branch, build-order, or traceability drift before asking the Orchestrator to review readiness.";
  }

  return {
    wpId,
    gitContext,
    packet,
    refinementPath,
    refinementExists,
    refinementReviewStatus,
    refinementValidation,
    enrichmentNeeded,
    clearlyCoversVerdict,
    userApprovalEvidence,
    refinementSignature,
    stubWpIds,
    microtaskCount,
    govKernelLink,
    taskBoardStatus: readTaskBoardStatus(wpId),
    claimCheck,
    traceabilityCheck,
    buildOrderCheck,
    topologyCheck,
    verdict,
    findings,
    artifactsReady,
    nextAction,
  };
}

function readinessArtifactPath(wpId) {
  return path.join(DEFAULT_RUNTIME_DIR, `${wpId}.md`);
}

function renderReadinessReport(state) {
  const lines = [
    "ACTIVATION_READINESS",
    `- WP_ID: ${state.wpId}`,
    `- VERDICT: ${state.verdict}`,
    `- TASK_BOARD_STATUS: ${state.taskBoardStatus || "<not found>"}`,
    `- PACKET_STATUS: ${state.packet.packetStatus || "<missing>"}`,
    `- CURRENT_WP_STATUS: ${state.packet.currentWpStatus || "<missing>"}`,
    `- WORKFLOW_LANE: ${state.packet.workflowLane || "<missing>"}`,
    `- EXECUTION_OWNER: ${state.packet.executionOwner || "<missing>"}`,
    `- STUBS_CREATED_OR_UPDATED: ${state.stubWpIds || "NONE"}`,
    `- LOCAL_BRANCH: ${state.packet.localBranch || "<missing>"}`,
    `- LOCAL_WORKTREE_DIR: ${state.packet.localWorktreeDir || "<missing>"}`,
    `- GOV_KERNEL_LINK: ${state.govKernelLink.status}`,
    `- REMOTE_BACKUP_BRANCH: ${state.packet.remoteBackupBranch || "<missing>"}`,
    `- BACKUP_PUSH_STATUS: ${state.packet.backupPushStatus || "<missing>"}`,
    `- MICROTASK_STATUS: ${state.microtaskCount > 0 ? `DECLARED:${state.microtaskCount}` : "NONE"}`,
    `- REFINEMENT_PATH: ${state.refinementPath}`,
    `- REFINEMENT_REVIEW_STATUS: ${state.refinementReviewStatus || "<missing>"}`,
    `- CLEARLY_COVERS_VERDICT: ${state.clearlyCoversVerdict || "<missing>"}`,
    `- ENRICHMENT_NEEDED: ${state.enrichmentNeeded || "<missing>"}`,
    `- USER_APPROVAL_EVIDENCE: ${state.userApprovalEvidence || "<missing>"}`,
    `- USER_SIGNATURE_REFINEMENT: ${state.refinementSignature || "<missing>"}`,
    `- USER_SIGNATURE_PACKET: ${state.packet.userSignature || "<missing>"}`,
    `- ARTIFACTS_READY: ${state.artifactsReady.length ? state.artifactsReady.join(" | ") : "NONE"}`,
    `- OUTSTANDING_ISSUES: ${state.findings.length ? "SEE_LIST" : "NONE"}`,
    `- NEXT_ORCHESTRATOR_ACTION: ${state.nextAction}`,
    "",
    "MECHANICAL_CHECKS",
    `- refinement-check: ${state.refinementValidation.ok ? "PASS" : "FAIL"}`,
    `- task-packet-claim-check: ${state.claimCheck.ok ? "PASS" : "FAIL"}`,
    `- wp-activation-traceability-check: ${state.traceabilityCheck.ok ? "PASS" : "FAIL"}`,
    `- build-order-check: ${state.buildOrderCheck.ok ? "PASS" : "FAIL"}`,
    `- wp-declared-topology-check: ${state.packet.packetExists ? (state.topologyCheck.ok ? "PASS" : "FAIL") : "NOT_CHECKED"}`,
  ];

  lines.push("");
  lines.push("OUTSTANDING_ISSUES");
  if (state.findings.length === 0) {
    lines.push("- NONE");
  } else {
    for (const finding of state.findings) {
      lines.push(`- ${finding}`);
    }
  }

  return `${lines.join("\n")}\n`;
}

function ensureRuntimeDir() {
  fs.mkdirSync(DEFAULT_RUNTIME_DIR, { recursive: true });
}

function writeReadinessReport(state) {
  ensureRuntimeDir();
  const artifactPath = readinessArtifactPath(state.wpId);
  fs.writeFileSync(artifactPath, renderReadinessReport(state), "utf8");
  return artifactPath;
}

function printStartup() {
  const gitContext = currentGitContext();
  const lines = [
    "ACTIVATION_MANAGER_STARTUP",
    `- ROLE: ${ACTOR_ROLE}`,
    "- AUTHORITY: .GOV/codex/Handshake_Codex_v1.4.md + ../handshake_main/AGENTS.md + .GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md",
    `- WORKTREE: ${gitContext.topLevel || "<unknown>"}`,
    `- BRANCH: ${gitContext.branch || "<unknown>"}`,
    "- SCOPE: refinement, approved spec enrichment, signature normalization/recording, packet hydration, microtask preparation, worktree preparation, activation readiness",
    "- REFINEMENT_STANDARD: match or exceed the old Orchestrator pre-launch quality bar, including research, primitive-index upkeep, matrix upkeep, appendix follow-through, and force-multiplier expansion.",
    "- STUB_DISCOVERY_RULE: create or update stubs when refinement, enrichment, or matrix upkeep discovers new required follow-up items.",
    "- HARD_STOP: no product code edits; no coder/validator launch; no final workflow authority",
    "- WORKFLOW_SPLIT: orchestrator-managed workflow requires ACTIVATION_MANAGER as the mandatory temporary pre-launch worker and governed pre-launch lane; manual workflow keeps pre-launch on ORCHESTRATOR.",
    "- HANDOFF_CHUNKING_RULE: long refinement or spec-enrichment output must be returned to the Orchestrator in bounded chunks; safe default is 4 blocks.",
    "- SIGNATURE_ROUND_TRIP: Activation Manager prepares the review-ready refinement/spec bundle, the Orchestrator collects approval evidence + one-time signature + coder choice, then Activation Manager resumes packet/worktree/backup/readiness work.",
    "- STARTUP_SEQUENCE:",
    "  1. just backup-status",
    "  2. just gov-check",
    "  3. just activation-manager prompt WP-{ID}",
    "  4. just activation-record-refinement WP-{ID}",
    "  5. just activation-record-signature WP-{ID} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} {Coder-A..Coder-Z}",
    "  6. just activation-create-task-packet WP-{ID} \"<context>\"",
    "  7. just activation-prepare-and-packet WP-{ID}",
    "  8. just activation-manager readiness WP-{ID} --write",
    "- CHECKPOINT_REQUIRED: SESSION_OPEN",
    `- Run: just repomem open "<what this activation session is about>" --role ${ACTOR_ROLE} [--wp WP-ID]`,
    "- RESUME_HINT: use `just activation-manager next WP-{ID}` to recompute the current activation state.",
  ];
  console.log(lines.join("\n"));
}

function printPrompt(state) {
  const lines = [
    `ROLE LOCK: You are the ${ACTOR_ROLE}.`,
    "Do not take Orchestrator, Coder, or Validator authority.",
    `WORKTREE: operate from wt-gov-kernel on branch gov_kernel.`,
    `WP_ID: ${state.wpId}`,
    `AUTHORITY: ${GOV_ROOT_REPO_REL}/codex/Handshake_Codex_v1.4.md + ../handshake_main/AGENTS.md + ${GOV_ROOT_REPO_REL}/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md`,
    "FOCUS: refinement, approved spec enrichment drafting, signature normalization/recording, packet hydration, microtask preparation, worktree preparation, and activation readiness.",
    "WORKFLOW SPLIT: ORCHESTRATOR_MANAGED requires ACTIVATION_MANAGER as the mandatory temporary pre-launch worker and governed pre-launch authoring lane; MANUAL_RELAY keeps pre-launch on ORCHESTRATOR.",
    "REFINEMENT STANDARD: match or exceed the old Orchestrator pre-launch quality bar, including research, primitive-index upkeep, matrix upkeep, appendix follow-through, force-multiplier expansion, and stub creation for new high-ROI discoveries.",
    "HANDOFF CHUNKING RULE: when returning long refinement or spec-enrichment text to the Orchestrator, split it into bounded chunks. Safe default: 4 blocks. Never paste the whole refinement in one message.",
    "SIGNATURE ROUND-TRIP: stop after the review-ready refinement/spec bundle and wait for the Orchestrator to return operator approval evidence, the one-time signature, and the selected Coder-A..Z owner before continuing packet/worktree/backup/readiness work.",
    "HARD BOUNDARIES: no product code edits; no coder or validator session launch; no final workflow status claims.",
    "REQUIRED OUTPUT: emit exactly one ACTIVATION_READINESS block when the bundle is ready, blocked, or needs repair.",
    `FIRST COMMANDS: just activation-manager next ${state.wpId} ; just activation-record-refinement ${state.wpId} ; just activation-manager readiness ${state.wpId} --write`,
  ];
  console.log(lines.join("\n"));
}

function printNext(state) {
  const readinessPath = readinessArtifactPath(state.wpId);
  const lines = [
    "ACTIVATION_MANAGER_NEXT",
    `- WP_ID: ${state.wpId}`,
    `- TASK_BOARD_STATUS: ${state.taskBoardStatus || "<not found>"}`,
    `- PACKET_STATUS: ${state.packet.packetStatus || "<missing>"}`,
    `- CURRENT_WP_STATUS: ${state.packet.currentWpStatus || "<missing>"}`,
    `- WORKFLOW_LANE: ${state.packet.workflowLane || "<missing>"}`,
    `- REFINEMENT_PATH: ${state.refinementPath}`,
    `- CLEARLY_COVERS_VERDICT: ${state.clearlyCoversVerdict || "<missing>"}`,
    `- ENRICHMENT_NEEDED: ${state.enrichmentNeeded || "<missing>"}`,
    `- USER_APPROVAL_EVIDENCE: ${state.userApprovalEvidence || "<missing>"}`,
    `- USER_SIGNATURE_REFINEMENT: ${state.refinementSignature || "<missing>"}`,
    `- USER_SIGNATURE_PACKET: ${state.packet.userSignature || "<missing>"}`,
    `- CURRENT_VERDICT: ${state.verdict}`,
    `- PRIMARY_RUNTIME_ARTIFACT: ${path.relative(REPO_ROOT, readinessPath).replace(/\\/g, "/")}`,
    "",
    "NEXT_COMMANDS",
  ];

  if (!state.refinementExists) {
    lines.push(`- Create or repair the refinement file at ${state.refinementPath}`);
    lines.push(`- just activation-create-task-packet ${state.wpId} "<context>"`);
  } else {
    lines.push(`- just generate-refinement-rubric`);
  }
  if (state.enrichmentNeeded === "YES") {
    lines.push(`- Perform the approved spec-enrichment pass for ${state.wpId}, then refresh the same refinement.`);
  } else if (!state.userApprovalEvidence || /^<pending>$/i.test(state.userApprovalEvidence)) {
    lines.push(`- Record USER_APPROVAL_EVIDENCE in the refinement as: APPROVE REFINEMENT ${state.wpId}`);
  } else if (!state.refinementSignature || /^<pending>$/i.test(state.refinementSignature)) {
    lines.push(`- just activation-record-signature ${state.wpId} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} {Coder-A..Coder-Z}`);
  }
  if (!state.packet.packetExists && state.refinementExists) {
    lines.push(`- just activation-create-task-packet ${state.wpId} "<context>"`);
  }
  lines.push(`- just activation-manager readiness ${state.wpId} --write`);
  lines.push(`- Review runtime artifact: ${path.relative(REPO_ROOT, readinessPath).replace(/\\/g, "/")}`);

  if (state.findings.length > 0) {
    lines.push("");
    lines.push("BLOCKERS");
    for (const finding of state.findings.slice(0, 12)) {
      lines.push(`- ${finding}`);
    }
  }

  console.log(lines.join("\n"));
}

function printJson(value) {
  console.log(JSON.stringify(value, null, 2));
}

const { action, wpId, write, json } = parseArgs(process.argv);

if (action === "startup") {
  printStartup();
  process.exit(0);
}

if (!wpId || !wpId.startsWith("WP-")) {
  fail("WP_ID is required for this action and must start with WP-", [
    `Example: just activation-manager ${action} WP-1-Example-v1`,
  ]);
}

const state = collectActivationState(wpId);

if (action === "prompt") {
  printPrompt(state);
  process.exit(0);
}

if (action === "next") {
  if (json) {
    printJson(state);
  } else {
    printNext(state);
  }
  process.exit(0);
}

if (action === "readiness") {
  const artifactPath = write ? writeReadinessReport(state) : "";
  if (json) {
    printJson({
      ...state,
      readinessArtifactPath: artifactPath || readinessArtifactPath(state.wpId),
      report: renderReadinessReport(state),
    });
  } else {
    console.log(renderReadinessReport(state));
    if (write) {
      console.log(`[ACTIVATION_MANAGER] wrote ${path.relative(REPO_ROOT, artifactPath).replace(/\\/g, "/")}`);
    }
  }
  process.exit(state.verdict === "READY_FOR_ORCHESTRATOR_REVIEW" ? 0 : 2);
}

usage();
