#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  cleanupPathSnapshot,
  createPathSnapshot,
  restorePathSnapshot,
} from "../../../roles_shared/scripts/lib/governance-transaction-utils.mjs";
import {
  lastGateLog,
  loadOrchestratorGateLogs,
  preparePacketTruthState,
  preparedWorktreeSyncState,
  printFindings,
  printLifecycle,
  printNextCommands,
  printOperatorAction,
  printState,
  taskBoardStatus,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  repoPathAbs,
  resolveOrchestratorGatesPath,
  resolveWorkPacketPath,
  WORK_PACKET_STORAGE_ROOT_REPO_REL,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { communicationPathsForWp } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { buildPhaseCheckCommand } from "../../../roles_shared/checks/phase-check-lib.mjs";
import {
  buildActivationManagerLaunchCommands,
  buildDownstreamGovernedLaunchCommands,
  buildManualRelayCommands,
  normalizeWorkflowLane,
} from "./lib/workflow-lane-guidance-lib.mjs";

const wpId = (process.argv[2] || "").trim();
const workflowLane = (process.argv[3] || "").trim();
const executionLane = (process.argv[4] || "").trim();

const TASK_BOARD_PATH = repoPathAbs(path.join(GOV_ROOT_REPO_REL, "roles_shared", "records", "TASK_BOARD.md"));
const TRACEABILITY_PATH = repoPathAbs(path.join(GOV_ROOT_REPO_REL, "roles_shared", "records", "WP_TRACEABILITY_REGISTRY.md"));
const BUILD_ORDER_PATH = repoPathAbs(path.join(GOV_ROOT_REPO_REL, "roles_shared", "records", "BUILD_ORDER.md"));
const ORCHESTRATOR_GATES_PATH = repoPathAbs(resolveOrchestratorGatesPath());
const LIVE_REVIEW_SCRIPT_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "scripts", "audit", "generate-post-run-audit-skeleton.mjs");

function usageAndExit() {
  console.error("Usage: node .GOV/roles/orchestrator/scripts/orchestrator-prepare-and-packet.mjs WP-{ID} [WORKFLOW_LANE] [EXECUTION_OWNER]");
  process.exit(1);
}

if (!wpId || !wpId.startsWith("WP-")) {
  usageAndExit();
}

function normalize(value) {
  return String(value || "").replace(/\\/g, "/");
}

function packetDirForWp(targetWpId) {
  return repoPathAbs(path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, targetWpId));
}

function packetPathForWp(targetWpId) {
  return normalize(
    resolveWorkPacketPath(targetWpId)?.packetPath
      || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, targetWpId, "packet.md"),
  );
}

function traceabilityActivePacket(baseWpId) {
  if (!fs.existsSync(TRACEABILITY_PATH)) return "";
  const lines = fs.readFileSync(TRACEABILITY_PATH, "utf8").split(/\r?\n/);
  for (const line of lines) {
    if (!line.trim().startsWith("|")) continue;
    if (line.includes("Base WP ID") || line.includes("---")) continue;
    const cells = line.split("|").slice(1, -1).map((cell) => cell.trim());
    if (cells.length < 2) continue;
    if (cells[0] === baseWpId) return cells[1];
  }
  return "";
}

function buildPrepareArgs() {
  const args = [wpId];
  if (workflowLane) args.push(workflowLane);
  if (executionLane) args.push(executionLane);
  return args;
}

function logProcessOutput(error) {
  const stdout = String(error?.stdout || "");
  const stderr = String(error?.stderr || "");
  if (stdout) process.stdout.write(stdout);
  if (stderr) process.stderr.write(stderr);
}

function runNodeStep(stepName, scriptRelativePath, args = []) {
  try {
    execFileSync(process.execPath, [scriptRelativePath, ...args], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    error.stepName = stepName;
    throw error;
  }
}

function ensureLiveWorkflowDossier(targetWpId) {
  try {
    const output = execFileSync(process.execPath, [LIVE_REVIEW_SCRIPT_PATH, targetWpId, "--mode", "live", "--auto-output"], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return normalize(String(output || "").trim());
  } catch (error) {
    logProcessOutput(error);
    return "";
  }
}

function rollbackAndExit(stepName, error, snapshot) {
  logProcessOutput(error);

  let rollbackMessage = "Activation state restored to the pre-run snapshot.";
  const findings = [`Failed step: ${stepName}`];

  try {
    restorePathSnapshot(snapshot);
    findings.push("Rollback: restored PREPARE, packet, task board, traceability, build order, and packet-scoped communications.");
  } catch (restoreError) {
    rollbackMessage = "Rollback failed after a transactional activation error. Manual repair is required.";
    findings.push(`Rollback failure: ${restoreError.message || String(restoreError)}`);
  } finally {
    cleanupPathSnapshot(snapshot);
  }

  printLifecycle({ wpId, stage: "PACKET_CREATE", next: "STOP" });
  printOperatorAction("NONE");
  printState(rollbackMessage);
  printFindings(findings);
  printNextCommands([
    `just gov-check`,
    `just orchestrator-next ${wpId}`,
  ]);
  process.exit(1);
}

function verifyTransactionalActivation() {
  const logs = loadOrchestratorGateLogs();
  const prepareEntry = lastGateLog(logs, wpId, "PREPARE");
  const issues = [];

  if (!prepareEntry) {
    issues.push("PREPARE record missing after activation.");
    return { ok: false, issues, syncState: null };
  }

  const packetTruth = preparePacketTruthState(wpId, prepareEntry, REPO_ROOT);
  if (!packetTruth.packetPresent) {
    issues.push(`Official packet missing after activation: ${packetTruth.packetPath}`);
  }
  if (!packetTruth.ok) {
    issues.push(...packetTruth.issues);
  }

  const boardStatus = taskBoardStatus(wpId);
  if (boardStatus !== "READY_FOR_DEV") {
    issues.push(`TASK_BOARD status is ${boardStatus || "<missing>"} instead of READY_FOR_DEV.`);
  }

  const communicationPaths = communicationPathsForWp(wpId);
  for (const [label, targetPath] of Object.entries({
    WP_COMMUNICATION_DIR: communicationPaths.dir,
    WP_THREAD_FILE: communicationPaths.threadFile,
    WP_RUNTIME_STATUS_FILE: communicationPaths.runtimeStatusFile,
    WP_RECEIPTS_FILE: communicationPaths.receiptsFile,
  })) {
    const absolutePath = repoPathAbs(targetPath);
    if (!fs.existsSync(absolutePath)) {
      issues.push(`${label} missing after activation: ${normalize(absolutePath)}`);
    }
  }

  const baseWpId = wpId.replace(/-v\d+$/i, "");
  if (baseWpId !== wpId) {
    const expectedPacketPath = packetPathForWp(wpId);
    const activePacketPath = traceabilityActivePacket(baseWpId);
    if (activePacketPath !== expectedPacketPath) {
      issues.push(`Traceability registry points to ${activePacketPath || "<missing>"} instead of ${expectedPacketPath}.`);
    }
  }

  return {
    ok: issues.length === 0,
    issues,
    prepareEntry,
    syncState: preparedWorktreeSyncState(wpId, prepareEntry, REPO_ROOT),
  };
}

function main() {
  const snapshot = createPathSnapshot([
    ORCHESTRATOR_GATES_PATH,
    packetDirForWp(wpId),
    TASK_BOARD_PATH,
    TRACEABILITY_PATH,
    BUILD_ORDER_PATH,
    repoPathAbs(communicationPathsForWp(wpId).dir),
  ], { label: `orchestrator-prepare-and-packet-${wpId}` });

  try {
    runNodeStep(
      "record-prepare",
      path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "checks", "orchestrator_gates.mjs"),
      ["prepare", ...buildPrepareArgs()],
    );

    runNodeStep(
      "create-task-packet",
      path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "create-task-packet.mjs"),
      [wpId],
    );

    runNodeStep(
      "task-board-set",
      path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "task-board-set.mjs"),
      [wpId, "READY_FOR_DEV"],
    );

    const baseWpId = wpId.replace(/-v\d+$/i, "");
    if (baseWpId !== wpId) {
      runNodeStep(
        "wp-traceability-set",
        path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "wp-traceability-set.mjs"),
        [baseWpId, wpId],
      );
    }

    runNodeStep(
      "build-order-sync",
      path.join(GOV_ROOT_REPO_REL, "roles_shared", "scripts", "build-order-sync.mjs"),
      [],
    );

    const verification = verifyTransactionalActivation();
    if (!verification.ok) {
      const error = new Error(`Activation verification failed for ${wpId}`);
      error.stdout = "";
      error.stderr = verification.issues.map((issue) => `[ORCHESTRATOR_PREPARE_AND_PACKET] ${issue}`).join("\n");
      rollbackAndExit("verify-activation", error, snapshot);
    }

    cleanupPathSnapshot(snapshot);
    const liveReviewPath = ensureLiveWorkflowDossier(wpId);

    const findings = [
      `PREPARE authority: ${verification.prepareEntry.workflow_lane} / ${verification.prepareEntry.execution_lane || verification.prepareEntry.coder_id}`,
      `Packet: ${packetPathForWp(wpId)}`,
      `Task Board: READY_FOR_DEV`,
      `Communications: ${communicationPathsForWp(wpId).dir}`,
    ];
    if (liveReviewPath) {
      findings.push(`Live workflow dossier: ${liveReviewPath}`);
    } else {
      findings.push("Live workflow dossier: NOT_CREATED (non-fatal; run `just live-smoketest-review-init`)");
    }

    const nextCommands = [];
    const normalizedWorkflowLane = normalizeWorkflowLane(verification.prepareEntry.workflow_lane);
    if (verification.syncState && !verification.syncState.ok) {
      printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
      printOperatorAction("NONE");
      printState(
        normalizedWorkflowLane === "ORCHESTRATOR_MANAGED"
          ? "Transactional activation completed, but the assigned WP worktree is still stale for Activation Manager-owned pre-launch."
          : "Transactional activation completed, but the assigned WP worktree is still stale for manual relay into implementation.",
      );
      printFindings([
        ...findings,
        ...verification.syncState.issues,
      ]);
      nextCommands.push(
        `# Repair ${verification.syncState.expectedBranch || "the assigned WP branch"} and ${verification.syncState.worktreeAbs || "the assigned WP worktree"} until they contain the official packet, current SPEC_CURRENT snapshot, current TASK_BOARD/traceability state, and current PREPARE record.`,
      );
      if (normalizedWorkflowLane === "ORCHESTRATOR_MANAGED") {
        nextCommands.push(...buildActivationManagerLaunchCommands(wpId));
      } else if (normalizedWorkflowLane === "MANUAL_RELAY") {
        nextCommands.push(...buildManualRelayCommands(wpId));
      } else {
        nextCommands.push(
          buildPhaseCheckCommand({ phase: "STARTUP", wpId, role: "CODER" }),
          ...buildDownstreamGovernedLaunchCommands(wpId),
        );
      }
      nextCommands.push(`just orchestrator-next ${wpId}`);
      printNextCommands(nextCommands);
      return;
    }

    printLifecycle({ wpId, stage: "DELEGATION", next: "DELEGATION" });
    printOperatorAction("NONE");
    printState(
      normalizedWorkflowLane === "ORCHESTRATOR_MANAGED"
        ? "Transactional activation completed and governance state is coherent for Activation Manager pre-launch."
        : "Transactional activation completed and governance state is coherent for manual relay.",
    );
    printFindings(findings);
    nextCommands.push(`cat ${packetPathForWp(wpId)}`);
    if (normalizedWorkflowLane === "ORCHESTRATOR_MANAGED") {
      nextCommands.push(...buildActivationManagerLaunchCommands(wpId));
    } else if (normalizedWorkflowLane === "MANUAL_RELAY") {
      nextCommands.push(...buildManualRelayCommands(wpId));
    } else {
      nextCommands.push(
        buildPhaseCheckCommand({ phase: "STARTUP", wpId, role: "CODER" }),
        ...buildDownstreamGovernedLaunchCommands(wpId),
      );
    }
    printNextCommands(nextCommands);
  } catch (error) {
    rollbackAndExit(error.stepName || "transaction", error, snapshot);
  }
}

main();
