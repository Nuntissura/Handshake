#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  addMinutes,
  deriveAuthorityKinds,
  normalize,
  parseJsonFile,
  validateRuntimeStatus,
  RUNTIME_STATUS_VALUES,
  NEXT_ACTOR_VALUES,
  VALIDATOR_TRIGGER_VALUES,
  ACTIVE_ROLE_VALUES,
} from "../lib/wp-communications-lib.mjs";
import { appendWpReceipt } from "./wp-receipt-append.mjs";
import { GOV_ROOT_REPO_REL } from "../lib/runtime-paths.mjs";

const PACKETS_DIR = path.join(GOV_ROOT_REPO_REL, "task_packets");

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function parsePacketStatus(text) {
  return (
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    ""
  ).trim() || "Ready for Dev";
}

function parseIntegerField(text, label, fallback) {
  const raw = parseSingleField(text, label);
  if (!raw) return fallback;
  const parsed = Number.parseInt(raw, 10);
  return Number.isInteger(parsed) ? parsed : fallback;
}

function requirePacketContext(wpId) {
  const packetPath = path.join(PACKETS_DIR, `${wpId}.md`);
  if (!fs.existsSync(packetPath)) {
    throw new Error(`Official packet not found: ${normalize(packetPath)}`);
  }
  const packetText = fs.readFileSync(packetPath, "utf8");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  if (!runtimeStatusFile) {
    throw new Error(`${normalize(packetPath)} does not declare WP_RUNTIME_STATUS_FILE`);
  }
  if (!receiptsFile) {
    throw new Error(`${normalize(packetPath)} does not declare WP_RECEIPTS_FILE`);
  }
  if (!fs.existsSync(runtimeStatusFile)) {
    throw new Error(`Runtime status file missing on disk: ${normalize(runtimeStatusFile)}`);
  }
  return {
    packetPath: normalize(packetPath),
    packetText,
    runtimeStatusFile: normalize(runtimeStatusFile),
    heartbeatIntervalMinutes: parseIntegerField(packetText, "HEARTBEAT_INTERVAL_MINUTES", 15),
    staleAfterMinutes: parseIntegerField(packetText, "STALE_AFTER_MINUTES", 45),
  };
}

function sessionStateForRuntimeStatus(runtimeStatus) {
  switch (runtimeStatus) {
    case "working":
      return "working";
    case "input_required":
      return "waiting";
    case "failed":
      return "blocked";
    case "completed":
    case "canceled":
      return "completed";
    default:
      return "idle";
  }
}

function nullableValue(value) {
  const raw = String(value ?? "").trim();
  if (!raw || /^null$/i.test(raw) || /^none$/i.test(raw) || /^n\/a$/i.test(raw)) return null;
  return raw;
}

export function recordWpHeartbeat({
  wpId,
  actorRole,
  actorSession,
  currentPhase,
  runtimeStatus,
  nextExpectedActor,
  waitingOn,
  validatorTrigger = "NONE",
  lastEvent = null,
  worktreeDir = null,
  nextExpectedSession = null,
  waitingOnSession = null,
} = {}) {
  const WP_ID = String(wpId || "").trim();
  const ACTOR_ROLE = String(actorRole || "").trim().toUpperCase();
  const ACTOR_SESSION = String(actorSession || "").trim();
  const CURRENT_PHASE = String(currentPhase || "").trim().toUpperCase();
  const RUNTIME_STATUS = String(runtimeStatus || "").trim();
  const NEXT_EXPECTED_ACTOR = String(nextExpectedActor || "").trim().toUpperCase();
  const WAITING_ON = String(waitingOn || "").trim();
  const VALIDATOR_TRIGGER = String(validatorTrigger || "NONE").trim().toUpperCase();
  const NEXT_EXPECTED_SESSION = nullableValue(nextExpectedSession);
  const WAITING_ON_SESSION = nullableValue(waitingOnSession);
  const now = new Date().toISOString();

  if (!WP_ID || !/^WP-/.test(WP_ID)) throw new Error("WP_ID is required");
  if (!ACTIVE_ROLE_VALUES.includes(ACTOR_ROLE)) throw new Error(`Invalid ACTOR_ROLE: ${ACTOR_ROLE}`);
  if (!ACTOR_SESSION) throw new Error("ACTOR_SESSION is required");
  if (!CURRENT_PHASE || !/^[A-Z][A-Z0-9_]*$/.test(CURRENT_PHASE)) throw new Error(`Invalid CURRENT_PHASE: ${CURRENT_PHASE}`);
  if (!RUNTIME_STATUS_VALUES.includes(RUNTIME_STATUS)) throw new Error(`Invalid RUNTIME_STATUS: ${RUNTIME_STATUS}`);
  if (!NEXT_ACTOR_VALUES.includes(NEXT_EXPECTED_ACTOR)) throw new Error(`Invalid NEXT_EXPECTED_ACTOR: ${NEXT_EXPECTED_ACTOR}`);
  if (!WAITING_ON) throw new Error("WAITING_ON is required");
  if (!VALIDATOR_TRIGGER_VALUES.includes(VALIDATOR_TRIGGER)) throw new Error(`Invalid VALIDATOR_TRIGGER: ${VALIDATOR_TRIGGER}`);

  const context = requirePacketContext(WP_ID);
  const runtimeStatusPath = context.runtimeStatusFile;
  const runtime = parseJsonFile(runtimeStatusPath);
  const stateBefore = `${runtime.runtime_status}/${runtime.current_phase}`;
  const { authorityKind, validatorRoleKind } = deriveAuthorityKinds({
    actorRole: ACTOR_ROLE,
    actorSession: ACTOR_SESSION,
    runtimeStatus: runtime,
  });

  runtime.current_packet_status = parsePacketStatus(context.packetText);
  runtime.runtime_status = RUNTIME_STATUS;
  runtime.current_phase = CURRENT_PHASE;
  runtime.next_expected_actor = NEXT_EXPECTED_ACTOR;
  runtime.next_expected_session = NEXT_EXPECTED_SESSION;
  runtime.waiting_on = WAITING_ON;
  runtime.waiting_on_session = WAITING_ON_SESSION;
  runtime.validator_trigger = VALIDATOR_TRIGGER;
  runtime.validator_trigger_reason = VALIDATOR_TRIGGER === "NONE" ? null : `${ACTOR_ROLE} signaled ${VALIDATOR_TRIGGER}`;
  runtime.ready_for_validation = ["READY_FOR_VALIDATION", "POST_WORK_PASS", "HANDOFF_READY"].includes(VALIDATOR_TRIGGER);
  runtime.ready_for_validation_reason = runtime.ready_for_validation ? runtime.validator_trigger_reason : null;
  runtime.last_event = String(lastEvent || `heartbeat_${ACTOR_ROLE.toLowerCase()}`);
  runtime.last_event_at = now;
  runtime.last_heartbeat_at = now;
  runtime.heartbeat_due_at = addMinutes(now, context.heartbeatIntervalMinutes);
  runtime.stale_after = addMinutes(now, context.staleAfterMinutes);

  const nextSessionState = sessionStateForRuntimeStatus(RUNTIME_STATUS);
  const filteredSessions = Array.isArray(runtime.active_role_sessions)
    ? runtime.active_role_sessions.filter((entry) => entry.role !== ACTOR_ROLE || entry.session_id !== ACTOR_SESSION)
    : [];
  filteredSessions.push({
    role: ACTOR_ROLE,
    session_id: ACTOR_SESSION,
    authority_kind: authorityKind,
    validator_role_kind: validatorRoleKind,
    worktree_dir: String(worktreeDir || context.packetText?.match?.(/^\s*-\s*LOCAL_WORKTREE_DIR\s*:\s*(.+)$/mi)?.[1] || runtime.current_worktree_dir || "<pending>").trim(),
    state: nextSessionState,
    last_heartbeat_at: now,
  });
  runtime.active_role_sessions = filteredSessions;

  const runtimeErrors = validateRuntimeStatus(runtime);
  if (runtimeErrors.length > 0) {
    throw new Error(`Runtime status validation failed: ${runtimeErrors.join("; ")}`);
  }

  fs.writeFileSync(runtimeStatusPath, `${JSON.stringify(runtime, null, 2)}\n`, "utf8");

  const nextDescriptor = NEXT_EXPECTED_SESSION ? `${NEXT_EXPECTED_ACTOR}:${NEXT_EXPECTED_SESSION}` : NEXT_EXPECTED_ACTOR;
  const waitingDescriptor = WAITING_ON_SESSION ? `${WAITING_ON} (${WAITING_ON_SESSION})` : WAITING_ON;
  const summary = `${ACTOR_ROLE} heartbeat: ${RUNTIME_STATUS}/${CURRENT_PHASE}; next=${nextDescriptor}; waiting_on=${waitingDescriptor}; validator_trigger=${VALIDATOR_TRIGGER}`;
  appendWpReceipt({
    wpId: WP_ID,
    actorRole: ACTOR_ROLE,
    actorSession: ACTOR_SESSION,
    receiptKind: "HEARTBEAT",
    summary,
    stateBefore,
    stateAfter: `${runtime.runtime_status}/${runtime.current_phase}`,
    refs: [runtimeStatusPath],
    worktreeDir: worktreeDir || runtime.current_worktree_dir || null,
    targetRole: runtime.next_expected_actor === "NONE" ? null : runtime.next_expected_actor,
    targetSession: runtime.next_expected_session,
    correlationId: `heartbeat:${WP_ID}:${ACTOR_SESSION}:${now}`,
  });

  return { runtimeStatusPath, runtime, summary };
}

function runCli() {
  const [wpId, actorRole, actorSession, currentPhase, runtimeStatus, nextExpectedActor, waitingOn, validatorTrigger, lastEvent, worktreeDir, nextExpectedSession, waitingOnSession] =
    process.argv.slice(2);

  if (!wpId || !actorRole || !actorSession || !currentPhase || !runtimeStatus || !nextExpectedActor || !waitingOn) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-heartbeat.mjs WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <CURRENT_PHASE> <RUNTIME_STATUS> <NEXT_EXPECTED_ACTOR> <WAITING_ON> [VALIDATOR_TRIGGER] [LAST_EVENT] [WORKTREE_DIR]"
      + " [NEXT_EXPECTED_SESSION] [WAITING_ON_SESSION]"
    );
    process.exit(1);
  }

  const result = recordWpHeartbeat({
    wpId,
    actorRole,
    actorSession,
    currentPhase,
    runtimeStatus,
    nextExpectedActor,
      waitingOn,
      validatorTrigger,
      lastEvent,
      worktreeDir,
      nextExpectedSession,
      waitingOnSession,
    });

  console.log(`[WP_HEARTBEAT] updated ${wpId}`);
  console.log(`- runtime: ${result.runtimeStatusPath}`);
  console.log(`- summary: ${result.summary}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
