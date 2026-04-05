import fs from "node:fs";
import path from "node:path";
import { REPO_ROOT, resolveWorkPacketPathAtRepo, WORK_PACKET_STORAGE_ROOT_REPO_REL } from "../lib/runtime-paths.mjs";

export const TERMINAL_SESSION_TASK_BOARD_STATUSES = new Set(["VALIDATED", "FAIL", "OUTDATED_ONLY", "ABANDONED", "SUPERSEDED"]);
const LOCAL_GOV_ROOT_REPO_REL = ".GOV";

function escapeRegex(value) {
  return String(value || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function parsePacketStatus(packetContent) {
  const match =
    String(packetContent || "").match(/^\s*-\s*\*\*Status:\*\*[ \t]*([^\r\n]+)[ \t]*$/mi) ||
    String(packetContent || "").match(/^\s*\*\*Status:\*\*[ \t]*([^\r\n]+)[ \t]*$/mi) ||
    String(packetContent || "").match(/^\s*Status:[ \t]*([^\r\n]+)[ \t]*$/mi);
  return match ? match[1].trim() : "";
}

function taskBoardStatusAtRepo(repoRoot, wpId) {
  const taskBoardPath = path.join(repoRoot, LOCAL_GOV_ROOT_REPO_REL, "roles_shared", "records", "TASK_BOARD.md");
  if (!fs.existsSync(taskBoardPath)) return "";
  const content = fs.readFileSync(taskBoardPath, "utf8");
  const match = content.match(
    new RegExp(`- \\*\\*\\[${escapeRegex(wpId)}\\]\\*\\* - \\[([^\\]]+)\\]`, "i"),
  );
  return match ? match[1].trim().toUpperCase() : "";
}

export function isTerminalSessionTaskBoardStatus(status) {
  return TERMINAL_SESSION_TASK_BOARD_STATUSES.has(String(status || "").trim().toUpperCase());
}

function packetPathAtRepo(repoRoot, wpId) {
  const resolved = resolveWorkPacketPathAtRepo(repoRoot, wpId, LOCAL_GOV_ROOT_REPO_REL);
  if (resolved?.packetAbsPath) {
    return {
      packetPathRel: resolved.packetPath,
      packetPathAbs: resolved.packetAbsPath,
    };
  }
  const fallbackRel = path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`);
  return {
    packetPathRel: fallbackRel,
    packetPathAbs: path.resolve(repoRoot, fallbackRel),
  };
}

export function evaluateSessionGovernanceState(repoRoot, sessionLike = {}) {
  const root = path.resolve(repoRoot || REPO_ROOT);
  const wpId = String(sessionLike.wp_id || sessionLike.wpId || "").trim();
  const localWorktreeDir = String(sessionLike.local_worktree_dir || sessionLike.localWorktreeDir || "").trim();
  const packetResolved = wpId ? packetPathAtRepo(root, wpId) : null;
  const packetPathRel = packetResolved?.packetPathRel || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`);
  const packetPathAbs = packetResolved?.packetPathAbs || path.resolve(root, packetPathRel);
  const packetExists = Boolean(wpId) && fs.existsSync(packetPathAbs);
  const packetStatus = packetExists ? parsePacketStatus(fs.readFileSync(packetPathAbs, "utf8")) : "";
  const taskBoardStatus = wpId ? taskBoardStatusAtRepo(root, wpId) : "";
  const terminalTaskBoardStatus = isTerminalSessionTaskBoardStatus(taskBoardStatus);
  const localWorktreeAbs = localWorktreeDir ? path.resolve(root, localWorktreeDir) : "";
  const localWorktreeExists = Boolean(localWorktreeAbs) && fs.existsSync(localWorktreeAbs);

  const launchBlockers = [];
  const steeringBlockers = [];

  if (!packetExists) {
    launchBlockers.push(`official packet is missing: ${packetPathRel}`);
    steeringBlockers.push(`official packet is missing: ${packetPathRel}`);
  }

  if (terminalTaskBoardStatus) {
    const reason = `task board status is terminal: ${taskBoardStatus}`;
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
    taskBoardStatus,
    terminalTaskBoardStatus,
    localWorktreeDir,
    localWorktreeAbs,
    localWorktreeExists,
    launchAllowed: launchBlockers.length === 0,
    steeringAllowed: steeringBlockers.length === 0,
    launchBlockers,
    steeringBlockers,
  };
}
