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
  return {
    current_packet_status: parsePacketStatus(packetText),
    main_containment_status: normalizeNoneLike(parseSingleField(packetText, "MAIN_CONTAINMENT_STATUS")),
    merged_main_commit: normalizeNoneLike(parseSingleField(packetText, "MERGED_MAIN_COMMIT")),
    main_containment_verified_at_utc: normalizeNoneLike(parseSingleField(packetText, "MAIN_CONTAINMENT_VERIFIED_AT_UTC")),
  };
}

export function syncRuntimeProjectionFromPacket(runtimeStatus, packetText, {
  eventName = "task_board_sync",
  eventAt = new Date().toISOString(),
} = {}) {
  const nextRuntime = { ...(runtimeStatus || {}) };
  const projection = parseRuntimeProjectionFromPacket(packetText);
  nextRuntime.current_packet_status = projection.current_packet_status;
  nextRuntime.main_containment_status = projection.main_containment_status;
  nextRuntime.merged_main_commit = projection.merged_main_commit;
  nextRuntime.main_containment_verified_at_utc = projection.main_containment_verified_at_utc;
  nextRuntime.last_event = eventName;
  nextRuntime.last_event_at = eventAt;
  return nextRuntime;
}
