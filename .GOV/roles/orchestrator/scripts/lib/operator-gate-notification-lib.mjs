import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  communicationPathsForWp,
  normalize,
  parseJsonlFile,
} from "../../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs } from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { appendWpNotification } from "../../../../roles_shared/scripts/wp/wp-notification-append.mjs";

const ORCHESTRATOR_NEXT_SCRIPT_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "orchestrator-next.mjs",
);

function parseSingleValueLine(text = "", label = "") {
  const match = String(text || "").match(new RegExp(`^${label}:\\s*(.+)\\s*$`, "mi"));
  return match ? String(match[1] || "").trim() : "";
}

function parseBulletValue(text = "", label = "") {
  const match = String(text || "").match(new RegExp(`^-\\s*${label}:\\s*(.+)\\s*$`, "mi"));
  return match ? String(match[1] || "").trim() : "";
}

export function parseOrchestratorLifecycleOutput(text = "") {
  const output = String(text || "");
  const stage = parseBulletValue(output, "STAGE");
  const next = parseBulletValue(output, "NEXT");
  const operatorAction = parseSingleValueLine(output, "OPERATOR_ACTION") || "NONE";
  const blockerClass = parseSingleValueLine(output, "BLOCKER_CLASS") || "NONE";
  const state = parseSingleValueLine(output, "STATE");
  const nextCommands = [];
  let inNextCommands = false;
  for (const line of output.split(/\r?\n/)) {
    if (/^NEXT_COMMANDS\b/i.test(line.trim())) {
      inNextCommands = true;
      continue;
    }
    if (!inNextCommands) continue;
    const match = line.match(/^\s*-\s*(.+)\s*$/);
    if (!match) {
      if (line.trim()) break;
      continue;
    }
    nextCommands.push(String(match[1] || "").trim());
  }
  return {
    stage,
    next,
    operatorAction,
    blockerClass,
    state,
    nextCommands,
  };
}

export function buildOperatorGateNotificationCandidate({
  wpId,
  lifecycle = null,
} = {}) {
  if (!wpId || !lifecycle) return null;
  const operatorAction = String(lifecycle.operatorAction || "").trim();
  const blockerClass = String(lifecycle.blockerClass || "").trim().toUpperCase();
  if (!operatorAction || operatorAction.toUpperCase() === "NONE") return null;
  if (!blockerClass || blockerClass === "NONE") return null;
  const stage = String(lifecycle.stage || "").trim().toUpperCase() || "UNKNOWN";
  const next = String(lifecycle.next || "").trim().toUpperCase() || "UNKNOWN";
  const state = String(lifecycle.state || "").trim();
  const summaryParts = [
    `OPERATOR_GATE: ${stage} -> ${next}`,
    blockerClass,
    operatorAction,
    state ? `State: ${state}` : "",
  ].filter(Boolean);
  return {
    wpId: String(wpId || "").trim(),
    sourceKind: "OPERATOR_GATE",
    targetRole: "OPERATOR",
    correlationId: `operator-gate:${String(wpId || "").trim()}:${blockerClass}:${stage}:${next}`,
    summary: summaryParts.join(" | "),
  };
}

export function operatorGateNotificationAlreadyPresent(wpId, candidate = null) {
  if (!wpId || !candidate?.correlationId) return false;
  const paths = communicationPathsForWp(wpId);
  const notificationsAbsPath = repoPathAbs(paths.notificationsFile);
  if (!fs.existsSync(notificationsAbsPath)) return false;
  const notifications = parseJsonlFile(paths.notificationsFile);
  return notifications.some((entry) =>
    String(entry?.target_role || "").trim().toUpperCase() === "OPERATOR"
    && String(entry?.source_kind || "").trim().toUpperCase() === String(candidate.sourceKind || "").trim().toUpperCase()
    && String(entry?.correlation_id || "").trim() === String(candidate.correlationId || "").trim()
    && normalize(entry?.summary || "") === normalize(candidate.summary || "")
  );
}

export function loadOperatorGateLifecycle(repoRoot, wpId) {
  const output = execFileSync(process.execPath, [ORCHESTRATOR_NEXT_SCRIPT_PATH, wpId], {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return parseOrchestratorLifecycleOutput(output);
}

export function emitOperatorGateNotificationIfNeeded({
  repoRoot,
  wpId,
  sourceSession = "",
} = {}) {
  if (!repoRoot || !wpId) {
    return { status: "NOT_APPLICABLE", reason: "MISSING_CONTEXT" };
  }
  try {
    const lifecycle = loadOperatorGateLifecycle(repoRoot, wpId);
    const candidate = buildOperatorGateNotificationCandidate({ wpId, lifecycle });
    if (!candidate) {
      return {
        status: "NOT_APPLICABLE",
        reason: "NO_OPERATOR_GATE",
        lifecycle,
      };
    }
    if (operatorGateNotificationAlreadyPresent(wpId, candidate)) {
      return {
        status: "ALREADY_PRESENT",
        reason: "DUPLICATE_CORRELATION",
        lifecycle,
        candidate,
      };
    }
    const communicationPaths = communicationPathsForWp(wpId);
    fs.mkdirSync(repoPathAbs(communicationPaths.dir), { recursive: true });
    const notification = appendWpNotification({
      wpId,
      sourceKind: candidate.sourceKind,
      sourceRole: "ORCHESTRATOR",
      sourceSession: String(sourceSession || "").trim() || "SESSION_CONTROL",
      targetRole: candidate.targetRole,
      correlationId: candidate.correlationId,
      summary: candidate.summary,
    }, { autoRelay: false });
    return {
      status: notification ? "EMITTED" : "SKIPPED",
      reason: notification ? "OPERATOR_GATE_APPENDED" : "NOTIFICATION_APPEND_SKIPPED",
      lifecycle,
      candidate,
      notification,
    };
  } catch (error) {
    return {
      status: "FAILED",
      reason: String(error?.message || error || "unknown error"),
    };
  }
}
