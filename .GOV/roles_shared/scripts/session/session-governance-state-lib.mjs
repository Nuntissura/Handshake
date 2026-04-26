import fs from "node:fs";
import path from "node:path";
import {
  REPO_ROOT,
  taskBoardPathAtRepo,
  WORK_PACKET_STORAGE_ROOT_REPO_REL,
  workPacketAbsPathAtRepo,
  workPacketPathAtRepo,
} from "../lib/runtime-paths.mjs";
import {
  isTerminalTaskBoardStatus,
  parsePacketStatus,
  parseTaskBoardStatus,
} from "../lib/wp-authority-projection-lib.mjs";
import { parseJsonFile } from "../lib/wp-communications-lib.mjs";
import { readExecutionPublicationView } from "../lib/wp-execution-state-lib.mjs";

export const TERMINAL_SESSION_TASK_BOARD_STATUSES = new Set(["VALIDATED", "FAIL", "OUTDATED_ONLY", "ABANDONED", "SUPERSEDED"]);
const LOCAL_GOV_ROOT_REPO_REL = ".GOV";

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function taskBoardStatusAtRepo(repoRoot, wpId) {
  const taskBoardPath = taskBoardPathAtRepo(repoRoot, LOCAL_GOV_ROOT_REPO_REL);
  if (!fs.existsSync(taskBoardPath)) return "";
  return parseTaskBoardStatus(fs.readFileSync(taskBoardPath, "utf8"), wpId);
}

export function isTerminalSessionTaskBoardStatus(status) {
  return TERMINAL_SESSION_TASK_BOARD_STATUSES.has(String(status || "").trim().toUpperCase())
    || isTerminalTaskBoardStatus(status);
}

function packetPathAtRepo(repoRoot, wpId) {
  return {
    packetPathRel: workPacketPathAtRepo(repoRoot, wpId, LOCAL_GOV_ROOT_REPO_REL)
      || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`),
    packetPathAbs: workPacketAbsPathAtRepo(repoRoot, wpId, LOCAL_GOV_ROOT_REPO_REL)
      || path.resolve(repoRoot, path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`)),
  };
}

export function evaluateSessionGovernanceState(repoRoot, sessionLike = {}) {
  const root = path.resolve(repoRoot || REPO_ROOT);
  const role = String(sessionLike.role || sessionLike.roleName || "").trim().toUpperCase();
  const wpId = String(sessionLike.wp_id || sessionLike.wpId || "").trim();
  const localWorktreeDir = String(sessionLike.local_worktree_dir || sessionLike.localWorktreeDir || "").trim();
  const packetResolved = wpId ? packetPathAtRepo(root, wpId) : null;
  const packetPathRel = packetResolved?.packetPathRel || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`);
  const packetPathAbs = packetResolved?.packetPathAbs || path.resolve(root, packetPathRel);
  const packetExists = Boolean(wpId) && fs.existsSync(packetPathAbs);
  const packetText = packetExists ? fs.readFileSync(packetPathAbs, "utf8") : "";
  const packetStatusArtifact = packetExists ? parsePacketStatus(packetText) : "";
  const taskBoardStatusArtifact = wpId ? taskBoardStatusAtRepo(root, wpId) : "";
  const runtimeStatusFile = packetExists ? parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE") : "";
  const runtimeStatusPathAbs = runtimeStatusFile ? path.resolve(root, runtimeStatusFile) : "";
  const runtimeProjection = runtimeStatusPathAbs && fs.existsSync(runtimeStatusPathAbs)
    ? readExecutionPublicationView({
        runtimeStatus: parseJsonFile(runtimeStatusPathAbs),
        packetStatus: packetStatusArtifact,
        taskBoardStatus: taskBoardStatusArtifact,
      })
    : readExecutionPublicationView({
        runtimeStatus: {},
        packetStatus: packetStatusArtifact,
        taskBoardStatus: taskBoardStatusArtifact,
      });
  const packetStatus = runtimeProjection.packet_status || packetStatusArtifact;
  const taskBoardStatus = runtimeProjection.task_board_status || taskBoardStatusArtifact;
  const terminalTaskBoardStatus = isTerminalSessionTaskBoardStatus(taskBoardStatus)
    || isTerminalSessionTaskBoardStatus(taskBoardStatusArtifact);
  const localWorktreeAbs = localWorktreeDir ? path.resolve(root, localWorktreeDir) : "";
  const localWorktreeExists = Boolean(localWorktreeAbs) && fs.existsSync(localWorktreeAbs);
  const packetProjectionDrift = Boolean(
    runtimeProjection.has_canonical_authority
    && packetStatusArtifact
    && runtimeProjection.packet_status
    && packetStatusArtifact !== runtimeProjection.packet_status
  );
  const taskBoardProjectionDrift = Boolean(
    runtimeProjection.has_canonical_authority
    && taskBoardStatusArtifact
    && runtimeProjection.task_board_status
    && taskBoardStatusArtifact !== runtimeProjection.task_board_status
  );

  const launchBlockers = [];
  const steeringBlockers = [];

  if (!packetExists && role !== "ACTIVATION_MANAGER" && role !== "MEMORY_MANAGER") {
    launchBlockers.push(`official packet is missing: ${packetPathRel}`);
    steeringBlockers.push(`official packet is missing: ${packetPathRel}`);
  }

  if (terminalTaskBoardStatus) {
    const reason = `task board status is terminal: ${isTerminalSessionTaskBoardStatus(taskBoardStatusArtifact) ? taskBoardStatusArtifact : taskBoardStatus}`;
    launchBlockers.push(reason);
    steeringBlockers.push(reason);
  }

  if (!localWorktreeDir) {
    steeringBlockers.push("local_worktree_dir is missing");
  } else if (!localWorktreeExists) {
    steeringBlockers.push(`assigned worktree is missing: ${localWorktreeDir}`);
  }

  return {
    wpId,
    packetPathRel,
    packetPathAbs,
    packetExists,
    packetStatus,
    packetStatusArtifact,
    taskBoardStatus,
    taskBoardStatusArtifact,
    terminalTaskBoardStatus,
    runtimeProjectionStatus: runtimeProjection.runtime_status || "",
    runtimePacketStatus: runtimeProjection.canonical_packet_status || "",
    runtimeTaskBoardStatus: runtimeProjection.canonical_task_board_status || "",
    packetProjectionDrift,
    taskBoardProjectionDrift,
    localWorktreeDir,
    localWorktreeAbs,
    localWorktreeExists,
    launchAllowed: launchBlockers.length === 0,
    steeringAllowed: steeringBlockers.length === 0,
    launchBlockers,
    steeringBlockers,
  };
}
