import { parseSignedScopeCompatibilityTruth } from "./signed-scope-compatibility-lib.mjs";

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

export function parsePacketStatus(packetText) {
  return (
    (String(packetText || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(packetText || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || ""
  ).trim() || "Ready for Dev";
}

function normalizeNoneLike(value) {
  const raw = String(value || "").trim();
  if (!raw || /^(NONE|N\/A|NA|NULL)$/i.test(raw)) return null;
  return raw;
}

export function parseRuntimeProjectionFromPacket(packetText) {
  const compatibility = parseSignedScopeCompatibilityTruth(packetText);
  return {
    current_packet_status: parsePacketStatus(packetText),
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
    nextRuntime.next_expected_session = null;
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
    nextRuntime.open_review_items = [];
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
  nextRuntime.current_packet_status = projection.current_packet_status;
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
  return nextRuntime;
}

export function evaluatePacketRuntimeProjectionDrift(packetText, runtimeStatus = {}, {
  communicationEvaluation = null,
} = {}) {
  const projection = parseRuntimeProjectionFromPacket(packetText);
  const issues = [];
  const runtimePhase = String(runtimeStatus?.current_phase || "").trim().toUpperCase();
  const runtimeStatusValue = String(runtimeStatus?.runtime_status || "").trim().toLowerCase();

  if (
    projection.current_packet_status
    && String(runtimeStatus?.current_packet_status || "").trim() !== String(projection.current_packet_status || "").trim()
  ) {
    issues.push(
      `runtime.current_packet_status (${runtimeStatus?.current_packet_status || "<missing>"}) does not match packet status (${projection.current_packet_status})`,
    );
  }

  if (
    projection.main_containment_status
    && String(runtimeStatus?.main_containment_status || "").trim().toUpperCase() !== String(projection.main_containment_status || "").trim().toUpperCase()
  ) {
    issues.push(
      `runtime.main_containment_status (${runtimeStatus?.main_containment_status || "<missing>"}) does not match packet MAIN_CONTAINMENT_STATUS (${projection.main_containment_status})`,
    );
  }

  if (projection.current_packet_status === "Done" || /^Validated \(/i.test(projection.current_packet_status || "")) {
    if (runtimePhase !== "STATUS_SYNC") {
      issues.push(`runtime.current_phase (${runtimeStatus?.current_phase || "<missing>"}) should be STATUS_SYNC once packet status is ${projection.current_packet_status}`);
    }
    if (runtimeStatusValue !== "completed") {
      issues.push(`runtime.runtime_status (${runtimeStatus?.runtime_status || "<missing>"}) should be completed once packet status is ${projection.current_packet_status}`);
    }
  }

  if (
    communicationEvaluation?.applicable
    && communicationEvaluation.ok
    && String(communicationEvaluation.state || "").trim().toUpperCase() === "COMM_OK"
  ) {
    if (runtimePhase === "BOOTSTRAP") {
      issues.push("runtime.current_phase is still BOOTSTRAP even though direct review is already complete");
    }
    if (String(projection.current_main_compatibility_status || "").trim().toUpperCase() === "NOT_RUN") {
      issues.push("packet still reports CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN even though the final direct-review lane is complete");
    }
  }

  return {
    ok: issues.length === 0,
    projection,
    issues,
  };
}
