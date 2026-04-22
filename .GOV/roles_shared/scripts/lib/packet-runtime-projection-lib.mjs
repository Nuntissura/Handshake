import { parseSignedScopeCompatibilityTruth } from "./signed-scope-compatibility-lib.mjs";
import {
  derivePacketMilestone,
  parsePacketStatus,
  taskBoardStatusForPacketStatus,
  runtimePhaseForMilestone,
} from "./wp-authority-projection-lib.mjs";
import {
  readExecutionPublicationView,
  syncRuntimeExecutionState,
} from "./wp-execution-state-lib.mjs";

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function normalizeDeclaredSessionValue(value) {
  const raw = String(value || "").trim();
  if (!raw || /^(<unassigned>|NONE|N\/A|NA|NULL)$/i.test(raw)) return null;
  return raw;
}

export { parsePacketStatus } from "./wp-authority-projection-lib.mjs";

function normalizeNoneLike(value) {
  const raw = String(value || "").trim();
  if (!raw || /^(NONE|N\/A|NA|NULL)$/i.test(raw)) return null;
  return raw;
}

export function parseRuntimeProjectionFromPacket(packetText) {
  const compatibility = parseSignedScopeCompatibilityTruth(packetText);
  const currentPacketStatus = parsePacketStatus(packetText);
  return {
    current_packet_status: currentPacketStatus,
    current_task_board_status: taskBoardStatusForPacketStatus(currentPacketStatus),
    wp_validator_of_record: normalizeDeclaredSessionValue(parseSingleField(packetText, "WP_VALIDATOR_OF_RECORD")),
    integration_validator_of_record: normalizeDeclaredSessionValue(parseSingleField(packetText, "INTEGRATION_VALIDATOR_OF_RECORD")),
    main_containment_status: normalizeNoneLike(parseSingleField(packetText, "MAIN_CONTAINMENT_STATUS")),
    merged_main_commit: normalizeNoneLike(parseSingleField(packetText, "MERGED_MAIN_COMMIT")),
    main_containment_verified_at_utc: normalizeNoneLike(parseSingleField(packetText, "MAIN_CONTAINMENT_VERIFIED_AT_UTC")),
    current_main_compatibility_status: normalizeNoneLike(compatibility.currentMainCompatibilityStatus),
    current_main_compatibility_baseline_sha: normalizeNoneLike(compatibility.currentMainCompatibilityBaselineSha),
    current_main_compatibility_verified_at_utc: normalizeNoneLike(compatibility.currentMainCompatibilityVerifiedAtUtc),
    packet_widening_decision: normalizeNoneLike(compatibility.packetWideningDecision),
    packet_widening_evidence: normalizeNoneLike(compatibility.packetWideningEvidence),
  };
}

function deriveRuntimeCloseoutState(projection = {}, currentRuntime = {}) {
  const status = String(projection.current_packet_status || "").trim();
  const containment = String(projection.main_containment_status || "").trim().toUpperCase();
  const nextRuntime = { ...(currentRuntime || {}) };

  if (status === "Done") {
    nextRuntime.current_phase = "STATUS_SYNC";
    nextRuntime.runtime_status = "completed";
    nextRuntime.next_expected_actor = containment === "MERGE_PENDING" ? "INTEGRATION_VALIDATOR" : "ORCHESTRATOR";
    nextRuntime.next_expected_session = containment === "MERGE_PENDING"
      ? (
        projection.integration_validator_of_record
        || normalizeDeclaredSessionValue(currentRuntime?.integration_validator_of_record)
        || normalizeDeclaredSessionValue(currentRuntime?.next_expected_session)
      )
      : null;
    nextRuntime.waiting_on = containment === "MERGE_PENDING" ? "MAIN_CONTAINMENT" : "STATUS_SYNC";
    nextRuntime.waiting_on_session = null;
    nextRuntime.validator_trigger = "NONE";
    nextRuntime.validator_trigger_reason = null;
    nextRuntime.ready_for_validation = false;
    nextRuntime.ready_for_validation_reason = null;
    nextRuntime.attention_required = false;
    return nextRuntime;
  }

  if (/^Validated \(/i.test(status)) {
    nextRuntime.current_phase = "STATUS_SYNC";
    nextRuntime.runtime_status = "completed";
    nextRuntime.next_expected_actor = "NONE";
    nextRuntime.next_expected_session = null;
    nextRuntime.waiting_on = "CLOSED";
    nextRuntime.waiting_on_session = null;
    nextRuntime.validator_trigger = "NONE";
    nextRuntime.validator_trigger_reason = null;
    nextRuntime.ready_for_validation = false;
    nextRuntime.ready_for_validation_reason = null;
    nextRuntime.attention_required = false;
    nextRuntime.current_files_touched = [];
    nextRuntime.active_role_sessions = [];
    nextRuntime.open_review_items = [];
    nextRuntime.route_anchor_state = null;
    nextRuntime.route_anchor_kind = null;
    nextRuntime.route_anchor_correlation_id = null;
    nextRuntime.route_anchor_target_role = null;
    nextRuntime.route_anchor_target_session = null;
    return nextRuntime;
  }

  return nextRuntime;
}

export function syncRuntimeProjectionFromPacket(runtimeStatus, packetText, {
  eventName = "task_board_sync",
  eventAt = new Date().toISOString(),
} = {}) {
  const projection = parseRuntimeProjectionFromPacket(packetText);
  const nextRuntime = deriveRuntimeCloseoutState(projection, runtimeStatus || {});
  const currentMilestone = derivePacketMilestone({
    packetStatus: projection.current_packet_status,
    currentMilestone: runtimeStatus?.current_milestone,
  });
  nextRuntime.current_packet_status = projection.current_packet_status;
  nextRuntime.current_task_board_status = projection.current_task_board_status;
  nextRuntime.current_milestone = currentMilestone;
  nextRuntime.current_phase = runtimePhaseForMilestone(currentMilestone, nextRuntime.current_phase || "BOOTSTRAP");
  nextRuntime.last_milestone_sync_at = eventAt;
  nextRuntime.wp_validator_of_record = projection.wp_validator_of_record;
  nextRuntime.integration_validator_of_record = projection.integration_validator_of_record;
  nextRuntime.main_containment_status = projection.main_containment_status;
  nextRuntime.merged_main_commit = projection.merged_main_commit;
  nextRuntime.main_containment_verified_at_utc = projection.main_containment_verified_at_utc;
  nextRuntime.current_main_compatibility_status = projection.current_main_compatibility_status;
  nextRuntime.current_main_compatibility_baseline_sha = projection.current_main_compatibility_baseline_sha;
  nextRuntime.current_main_compatibility_verified_at_utc = projection.current_main_compatibility_verified_at_utc;
  nextRuntime.packet_widening_decision = projection.packet_widening_decision;
  nextRuntime.packet_widening_evidence = projection.packet_widening_evidence;
  nextRuntime.last_event = eventName;
  nextRuntime.last_event_at = eventAt;
  return syncRuntimeExecutionState(nextRuntime, {
    eventName,
    eventAt,
    checkpointKind: "PACKET_SYNC",
  });
}

const DRIFT_OWNER_PRIORITY = {
  PACKET_CLOSEOUT_TRUTH: 0,
  RUNTIME_PROJECTION: 1,
};

function packetRuntimeDriftDetail(owner, surface, message) {
  return { owner, surface, message };
}

function orderDriftOwners(ownerSet) {
  return Array.from(ownerSet).sort((left, right) => {
    const leftPriority = DRIFT_OWNER_PRIORITY[left] ?? Number.MAX_SAFE_INTEGER;
    const rightPriority = DRIFT_OWNER_PRIORITY[right] ?? Number.MAX_SAFE_INTEGER;
    if (leftPriority !== rightPriority) return leftPriority - rightPriority;
    return String(left).localeCompare(String(right));
  });
}

function ownerLabel(owner) {
  switch (owner) {
    case "PACKET_CLOSEOUT_TRUTH":
      return "packet closeout truth";
    case "RUNTIME_PROJECTION":
      return "runtime projection";
    default:
      return String(owner || "").trim().toLowerCase().replaceAll("_", " ");
  }
}

function buildDriftOwnerSummary(ownerClasses) {
  if (!Array.isArray(ownerClasses) || ownerClasses.length === 0) {
    return null;
  }
  if (ownerClasses.length === 1) {
    return `Drift is owned by ${ownerLabel(ownerClasses[0])}; repair ${ownerLabel(ownerClasses[0])} first.`;
  }
  return `Drift spans ${ownerClasses.map(ownerLabel).join(" and ")}; repair ${ownerClasses.map(ownerLabel).join(" -> ")} in that order.`;
}

export function evaluatePacketRuntimeProjectionDrift(packetText, runtimeStatus = {}, {
  communicationEvaluation = null,
} = {}) {
  const publication = readExecutionPublicationView({
    runtimeStatus,
    packetStatus: parsePacketStatus(packetText),
  });
  runtimeStatus = publication.runtime;
  const projection = parseRuntimeProjectionFromPacket(packetText);
  const issues = [];
  const issueDetails = [];
  const ownerClasses = new Set();
  const canonicalAuthorityOwnsPublication = publication.has_canonical_authority;
  const runtimePhase = String(runtimeStatus?.current_phase || "").trim().toUpperCase();
  const runtimeStatusValue = String(runtimeStatus?.runtime_status || "").trim().toLowerCase();
  const effectivePacketStatus = String(publication.packet_status || projection.current_packet_status || "").trim();

  if (
    projection.current_packet_status
    && String(runtimeStatus?.current_packet_status || "").trim() !== String(projection.current_packet_status || "").trim()
  ) {
    const message = canonicalAuthorityOwnsPublication
      ? `packet status (${projection.current_packet_status}) does not match canonical execution publication status (${runtimeStatus?.current_packet_status || "<missing>"})`
      : `runtime.current_packet_status (${runtimeStatus?.current_packet_status || "<missing>"}) does not match packet status (${projection.current_packet_status})`;
    issues.push(message);
    ownerClasses.add(canonicalAuthorityOwnsPublication ? "PACKET_CLOSEOUT_TRUTH" : "RUNTIME_PROJECTION");
    issueDetails.push(
      packetRuntimeDriftDetail(
        canonicalAuthorityOwnsPublication ? "PACKET_CLOSEOUT_TRUTH" : "RUNTIME_PROJECTION",
        canonicalAuthorityOwnsPublication ? "packet.Status" : "runtime.current_packet_status",
        message,
      ),
    );
  }

  if (
    projection.current_task_board_status
    && String(runtimeStatus?.current_task_board_status || "").trim().toUpperCase() !== String(projection.current_task_board_status || "").trim().toUpperCase()
  ) {
    const message = canonicalAuthorityOwnsPublication
      ? `packet/task-board projection (${projection.current_task_board_status}) does not match canonical execution publication status (${runtimeStatus?.current_task_board_status || "<missing>"})`
      : `runtime.current_task_board_status (${runtimeStatus?.current_task_board_status || "<missing>"}) does not match packet/task-board projection (${projection.current_task_board_status})`;
    issues.push(message);
    ownerClasses.add(canonicalAuthorityOwnsPublication ? "PACKET_CLOSEOUT_TRUTH" : "RUNTIME_PROJECTION");
    issueDetails.push(
      packetRuntimeDriftDetail(
        canonicalAuthorityOwnsPublication ? "PACKET_CLOSEOUT_TRUTH" : "RUNTIME_PROJECTION",
        canonicalAuthorityOwnsPublication ? "packet.STATUS_HANDOFF.Current_WP_STATUS" : "runtime.current_task_board_status",
        message,
      ),
    );
  }

  if (
    projection.main_containment_status
    && String(runtimeStatus?.main_containment_status || "").trim().toUpperCase() !== String(projection.main_containment_status || "").trim().toUpperCase()
  ) {
    const message = canonicalAuthorityOwnsPublication
      ? `packet MAIN_CONTAINMENT_STATUS (${projection.main_containment_status}) does not match canonical execution publication status (${runtimeStatus?.main_containment_status || "<missing>"})`
      : `runtime.main_containment_status (${runtimeStatus?.main_containment_status || "<missing>"}) does not match packet MAIN_CONTAINMENT_STATUS (${projection.main_containment_status})`;
    issues.push(message);
    ownerClasses.add(canonicalAuthorityOwnsPublication ? "PACKET_CLOSEOUT_TRUTH" : "RUNTIME_PROJECTION");
    issueDetails.push(
      packetRuntimeDriftDetail(
        canonicalAuthorityOwnsPublication ? "PACKET_CLOSEOUT_TRUTH" : "RUNTIME_PROJECTION",
        canonicalAuthorityOwnsPublication ? "packet.MAIN_CONTAINMENT_STATUS" : "runtime.main_containment_status",
        message,
      ),
    );
  }

  if (effectivePacketStatus === "Done" || /^Validated \(/i.test(effectivePacketStatus)) {
    if (runtimePhase !== "STATUS_SYNC") {
      const message = `runtime.current_phase (${runtimeStatus?.current_phase || "<missing>"}) should be STATUS_SYNC once packet status is ${effectivePacketStatus}`;
      issues.push(message);
      ownerClasses.add("RUNTIME_PROJECTION");
      issueDetails.push(
        packetRuntimeDriftDetail(
          "RUNTIME_PROJECTION",
          "runtime.current_phase",
          message,
        ),
      );
    }
    if (runtimeStatusValue !== "completed") {
      const message = `runtime.runtime_status (${runtimeStatus?.runtime_status || "<missing>"}) should be completed once packet status is ${effectivePacketStatus}`;
      issues.push(message);
      ownerClasses.add("RUNTIME_PROJECTION");
      issueDetails.push(
        packetRuntimeDriftDetail(
          "RUNTIME_PROJECTION",
          "runtime.runtime_status",
          message,
        ),
      );
    }
  }

  if (
    communicationEvaluation?.applicable
    && communicationEvaluation.ok
    && String(communicationEvaluation.state || "").trim().toUpperCase() === "COMM_OK"
  ) {
    if (runtimePhase === "BOOTSTRAP") {
      const message = "runtime.current_phase is still BOOTSTRAP even though direct review is already complete";
      issues.push(message);
      ownerClasses.add("RUNTIME_PROJECTION");
      issueDetails.push(
        packetRuntimeDriftDetail(
          "RUNTIME_PROJECTION",
          "runtime.current_phase",
          message,
        ),
      );
    }
    if (String(runtimeStatus?.current_milestone || "").trim().toUpperCase() !== "VERDICT") {
      const message = `runtime.current_milestone (${runtimeStatus?.current_milestone || "<missing>"}) should be VERDICT once the direct-review lane is complete`;
      issues.push(message);
      ownerClasses.add("RUNTIME_PROJECTION");
      issueDetails.push(
        packetRuntimeDriftDetail(
          "RUNTIME_PROJECTION",
          "runtime.current_milestone",
          message,
        ),
      );
    }
    if (String(projection.current_main_compatibility_status || "").trim().toUpperCase() === "NOT_RUN") {
      const message = "packet still reports CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN even though the final direct-review lane is complete";
      issues.push(message);
      ownerClasses.add("PACKET_CLOSEOUT_TRUTH");
      issueDetails.push(
        packetRuntimeDriftDetail(
          "PACKET_CLOSEOUT_TRUTH",
          "packet.CURRENT_MAIN_COMPATIBILITY_STATUS",
          message,
        ),
      );
    }
  }

  const orderedOwners = orderDriftOwners(ownerClasses);

  return {
    ok: issues.length === 0,
    projection,
    publication,
    issues,
    issue_details: issueDetails,
    owner_classes: orderedOwners,
    repair_order: orderedOwners,
    owner_summary: buildDriftOwnerSummary(orderedOwners),
  };
}
