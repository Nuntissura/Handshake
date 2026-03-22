#!/usr/bin/env node

import fs from "node:fs";
import {
  activeOrchestratorCandidates,
  currentGitContext,
  lastGateLog,
  loadOrchestratorGateLogs,
  packetExists,
  preparePacketTruthState,
  preparedWorktreeSyncState,
  resolvePrepareWorktreeAbs,
  taskBoardStatus,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";

const ACTIVE_TASK_BOARD_STATUSES = new Set(["READY_FOR_DEV", "IN_PROGRESS", "BLOCKED"]);

function fail(message, details = []) {
  console.error(`[ORCHESTRATOR_STARTUP_TRUTH_CHECK] ${message}`);
  for (const detail of details) console.error(`- ${detail}`);
  process.exit(1);
}

function uniqueSorted(values) {
  return [...new Set(values.filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

function worktreeExists(worktreeAbs) {
  try {
    return fs.existsSync(worktreeAbs);
  } catch {
    return false;
  }
}

function main() {
  const repoRoot = currentGitContext().topLevel || process.cwd();
  const gateLogs = loadOrchestratorGateLogs();
  const wpIds = uniqueSorted(activeOrchestratorCandidates(gateLogs).map((entry) => entry.wpId));
  const violations = [];

  for (const wpId of wpIds) {
    const boardStatus = taskBoardStatus(wpId) || "<none>";
    const prepareEntry = lastGateLog(gateLogs, wpId, "PREPARE");
    const hasPacket = packetExists(wpId);
    const boardSaysActive = ACTIVE_TASK_BOARD_STATUSES.has(boardStatus);

    if (boardSaysActive && !hasPacket) {
      violations.push(`${wpId}: TASK_BOARD is ${boardStatus}, but the official packet is missing`);
    }
    if (boardSaysActive && !prepareEntry) {
      violations.push(`${wpId}: TASK_BOARD is ${boardStatus}, but no PREPARE authority exists in ORCHESTRATOR_GATES`);
    }
    if (!prepareEntry) continue;

    const packetTruth = preparePacketTruthState(wpId, prepareEntry, repoRoot);
    if (!packetTruth.ok) {
      for (const issue of packetTruth.issues) {
        violations.push(`${wpId}: ${issue}`);
      }
    }

    if (!packetTruth.packetPresent) {
      const worktreeAbs = resolvePrepareWorktreeAbs(prepareEntry, repoRoot);
      if (!worktreeAbs) {
        violations.push(`${wpId}: PREPARE is missing worktree_dir`);
      } else if (!worktreeExists(worktreeAbs)) {
        violations.push(`${wpId}: PREPARE points to a missing worktree: ${worktreeAbs}`);
      }
      continue;
    }

    const syncState = preparedWorktreeSyncState(wpId, prepareEntry, repoRoot);
    if (!syncState.ok) {
      for (const issue of syncState.issues) {
        violations.push(`${wpId}: ${issue}`);
      }
    }
  }

  if (violations.length > 0) {
    fail("Active orchestrator authority surfaces are split; fix startup truth before more execution proceeds.", [
      `checked_wps=${wpIds.length || 0}`,
      ...violations,
      "Run `just orchestrator-next WP-{ID}` on the failing WPs and repair STATUS_SYNC before launching more work.",
    ]);
  }

  console.log("orchestrator-startup-truth-check ok");
  console.log(`- checked_wps: ${wpIds.length}`);
}

main();
