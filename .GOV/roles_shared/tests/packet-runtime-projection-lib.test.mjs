import assert from "node:assert/strict";
import test from "node:test";
import {
  parseRuntimeProjectionFromPacket,
  syncRuntimeProjectionFromPacket,
} from "../scripts/lib/packet-runtime-projection-lib.mjs";

function packetFixture({
  status = "Validated (PASS)",
  containment = "CONTAINED_IN_MAIN",
  merged = "0123456789abcdef0123456789abcdef01234567",
  verifiedAt = "2026-03-26T12:00:00Z",
} = {}) {
  return [
    "## METADATA",
    `- **Status:** ${status}`,
    `- MAIN_CONTAINMENT_STATUS: ${containment}`,
    `- MERGED_MAIN_COMMIT: ${merged}`,
    `- MAIN_CONTAINMENT_VERIFIED_AT_UTC: ${verifiedAt}`,
  ].join("\n");
}

test("parseRuntimeProjectionFromPacket normalizes terminal packet truth", () => {
  const projection = parseRuntimeProjectionFromPacket(packetFixture());
  assert.deepEqual(projection, {
    current_packet_status: "Validated (PASS)",
    main_containment_status: "CONTAINED_IN_MAIN",
    merged_main_commit: "0123456789abcdef0123456789abcdef01234567",
    main_containment_verified_at_utc: "2026-03-26T12:00:00Z",
  });
});

test("syncRuntimeProjectionFromPacket updates runtime projection fields and event metadata", () => {
  const runtime = syncRuntimeProjectionFromPacket(
    {
      current_packet_status: "Done",
      main_containment_status: "MERGE_PENDING",
      merged_main_commit: null,
      main_containment_verified_at_utc: null,
      last_event: "old",
      last_event_at: "2026-03-25T10:00:00Z",
    },
    packetFixture(),
    {
      eventName: "task_board_sync",
      eventAt: "2026-03-26T13:00:00Z",
    },
  );

  assert.equal(runtime.current_packet_status, "Validated (PASS)");
  assert.equal(runtime.main_containment_status, "CONTAINED_IN_MAIN");
  assert.equal(runtime.merged_main_commit, "0123456789abcdef0123456789abcdef01234567");
  assert.equal(runtime.main_containment_verified_at_utc, "2026-03-26T12:00:00Z");
  assert.equal(runtime.last_event, "task_board_sync");
  assert.equal(runtime.last_event_at, "2026-03-26T13:00:00Z");
});
