import { parsePacketStatus } from "./packet-runtime-projection-lib.mjs";
import {
  derivePacketMilestone,
  isTerminalPacketStatus,
  packetStatusForCommunicationState,
  runtimePhaseForMilestone,
  taskBoardStatusForPacketStatus,
} from "./wp-authority-projection-lib.mjs";

function replaceCurrentStateField(text, label, value) {
  const re = new RegExp(`^(\\s*${label}\\s*:\\s*)(.+)\\s*$`, "mi");
  if (!re.test(String(text || ""))) {
    throw new Error(`Missing CURRENT_STATE field in packet: ${label}`);
  }
  return String(text || "").replace(re, `$1${value}`);
}

function replacePacketStatusField(text, value) {
  const re = /^(\s*-\s*\*\*Status:\*\*\s*)(.+)\s*$/mi;
  if (!re.test(String(text || ""))) {
    throw new Error("Missing canonical packet status field");
  }
  return String(text || "").replace(re, `$1${value}`);
}

function normalizeState(value) {
  return String(value || "").trim().toUpperCase();
}

function currentStateForEvaluation(evaluationState, autoRoute = {}, evaluation = {}) {
  const authoritativeReceiptKind = String(evaluation?.latestValidatorAssessment?.receiptKind || "VALIDATOR_REVIEW").trim() || "VALIDATOR_REVIEW";
  switch (normalizeState(evaluationState)) {
    case "COMM_MISSING_KICKOFF":
      return {
        verdict: "PENDING",
        blockers: "Awaiting WP validator kickoff for the governed direct-review lane.",
        next: "WP_VALIDATOR records VALIDATOR_KICKOFF for the active packet.",
      };
    case "COMM_WAITING_FOR_INTENT":
      return {
        verdict: "PENDING",
        blockers: "Awaiting CODER intent reply to the validator kickoff.",
        next: "CODER records CODER_INTENT with implementation order and proof plan.",
      };
    case "COMM_WAITING_FOR_INTENT_CHECKPOINT":
      return {
        verdict: "PENDING",
        blockers: "Bootstrap and skeleton clearance now belongs to the WP validator; coder intent must be explicitly cleared before implementation hardens or full handoff proceeds.",
        next: "WP_VALIDATOR reviews CODER_INTENT and records SPEC_GAP / VALIDATOR_QUERY for missing signed surfaces or proof, or VALIDATOR_RESPONSE to clear bootstrap/skeleton intent review.",
      };
    case "COMM_WAITING_FOR_HANDOFF":
      if (Number(evaluation?.counts?.overlapOpenReviewItems || 0) > 0) {
        return {
          verdict: "PENDING",
          blockers: "Implementation is in progress; a completed previous microtask is awaiting WP validator overlap review while the coder continues the current bounded microtask.",
          next: "WP_VALIDATOR reviews the open overlap microtask item while CODER completes the current microtask before any loop-back or further forward advance.",
        };
      }
      return {
        verdict: "PENDING",
        blockers: "Implementation is in progress; awaiting coder handoff to WP validator.",
        next: "CODER completes in-scope work and records CODER_HANDOFF with proof.",
      };
    case "COMM_REPAIR_REQUIRED":
      return {
        verdict: "PENDING",
        blockers: `WP validator review requires coder remediation; see the authoritative latest ${authoritativeReceiptKind} receipt for the active handoff findings.`,
        next: `CODER repairs against the authoritative latest ${authoritativeReceiptKind}, commits the reviewable state, and re-records CODER_HANDOFF with proof.`,
      };
    case "COMM_WAITING_FOR_REVIEW":
      return {
        verdict: "PENDING",
        blockers: "Awaiting WP validator review of the latest coder handoff.",
        next: "WP_VALIDATOR reviews the latest CODER_HANDOFF and records VALIDATOR_REVIEW.",
      };
    case "COMM_WAITING_FOR_FINAL_REVIEW":
      return {
        verdict: "PENDING",
        blockers: "Awaiting the final direct review exchange with INTEGRATION_VALIDATOR.",
        next: "CODER initiates the final direct review exchange with INTEGRATION_VALIDATOR.",
      };
    case "COMM_BLOCKED_OPEN_ITEMS":
      return {
        verdict: "PENDING",
        blockers: "Open review items still block governed direct-review progression; see WP communications for the authoritative pending item.",
        next: `${String(autoRoute.nextExpectedActor || "ORCHESTRATOR").trim() || "ORCHESTRATOR"} resolves the pending review item and records the matching response receipt.`,
      };
    case "COMM_OK":
      return {
        verdict: "PENDING",
        blockers: "NONE",
        next: "ORCHESTRATOR advances verdict progression and integration closeout from the authoritative completed direct-review lane.",
      };
    case "COMM_MISCONFIGURED":
      return {
        verdict: "PENDING",
        blockers: "The governed direct-review communication contract is misconfigured and must be repaired before work can continue.",
        next: "ORCHESTRATOR repairs the communication contract and restores a valid route state.",
      };
    case "COMM_WORKFLOW_INVALID":
      return {
        verdict: "PENDING",
        blockers: "A workflow invalidity is active on this packet; governed lane truth must be repaired before further progression.",
        next: "ORCHESTRATOR resolves the active workflow invalidity and restarts the governed lane from truthful state.",
      };
    default:
      return {
        verdict: "PENDING",
        blockers: String(autoRoute.waitingOn || "Governed lane state requires attention."),
        next: `${String(autoRoute.nextExpectedActor || "ORCHESTRATOR").trim() || "ORCHESTRATOR"} handles the next governed action.`,
      };
  }
}

export function deriveWpReviewPacketProjection({
  evaluation,
  autoRoute = {},
  packetText = "",
} = {}) {
  if (!evaluation?.applicable) return null;

  const currentPacketStatus = parsePacketStatus(packetText);
  const packetStatus = packetStatusForCommunicationState(evaluation.state, currentPacketStatus);
  const currentState = currentStateForEvaluation(evaluation.state, autoRoute, evaluation);

  return {
    packetStatus,
    taskBoardStatus: taskBoardStatusForPacketStatus(packetStatus),
    taskBoardReason: packetStatus === "Blocked" ? currentState.blockers : "",
    verdict: currentState.verdict,
    blockers: currentState.blockers,
    next: currentState.next,
  };
}

export function applyWpReviewPacketProjection(packetText, projection = {}) {
  let nextText = String(packetText || "");
  if (projection.packetStatus) {
    nextText = replacePacketStatusField(nextText, projection.packetStatus);
  }
  if (projection.verdict) {
    nextText = replaceCurrentStateField(nextText, "Verdict", projection.verdict);
  }
  if (projection.blockers) {
    nextText = replaceCurrentStateField(nextText, "Blockers", projection.blockers);
  }
  if (projection.next) {
    nextText = replaceCurrentStateField(nextText, "Next", projection.next);
  }
  return nextText;
}

export function applyWpReviewRuntimeProjection(runtimeStatus, {
  evaluation,
} = {}) {
  if (!evaluation?.applicable) return { ...(runtimeStatus || {}) };

  const nextRuntime = { ...(runtimeStatus || {}) };
  if (isTerminalPacketStatus(nextRuntime.current_packet_status)) {
    return nextRuntime;
  }

  switch (normalizeState(evaluation.state)) {
    case "COMM_MISSING_KICKOFF":
    case "COMM_WAITING_FOR_INTENT":
      nextRuntime.runtime_status = "submitted";
      nextRuntime.current_phase = "BOOTSTRAP";
      break;
    case "COMM_WAITING_FOR_INTENT_CHECKPOINT":
      nextRuntime.runtime_status = "working";
      nextRuntime.current_phase = "BOOTSTRAP";
      break;
    case "COMM_WAITING_FOR_HANDOFF":
    case "COMM_REPAIR_REQUIRED":
      nextRuntime.runtime_status = "working";
      nextRuntime.current_phase = "IMPLEMENTATION";
      break;
    case "COMM_WAITING_FOR_REVIEW":
    case "COMM_WAITING_FOR_FINAL_REVIEW":
    case "COMM_BLOCKED_OPEN_ITEMS":
    case "COMM_OK":
      nextRuntime.runtime_status = "working";
      nextRuntime.current_phase = "VALIDATION";
      break;
    case "COMM_MISCONFIGURED":
    case "COMM_WORKFLOW_INVALID":
      nextRuntime.runtime_status = "input_required";
      nextRuntime.current_phase = "WORKFLOW_REPAIR";
      break;
    default:
      break;
  }

  const milestone = derivePacketMilestone({
    packetStatus: nextRuntime.current_packet_status,
    communicationState: evaluation.state,
    currentMilestone: nextRuntime.current_milestone,
  });
  nextRuntime.current_task_board_status = taskBoardStatusForPacketStatus(nextRuntime.current_packet_status);
  nextRuntime.current_milestone = milestone;
  nextRuntime.current_phase = runtimePhaseForMilestone(milestone, nextRuntime.current_phase || "BOOTSTRAP");
  nextRuntime.last_milestone_sync_at = nextRuntime.last_event_at || nextRuntime.last_milestone_sync_at || new Date().toISOString();

  return nextRuntime;
}
