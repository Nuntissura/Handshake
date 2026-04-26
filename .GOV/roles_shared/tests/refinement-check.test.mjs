import test from "node:test";
import assert from "node:assert/strict";

import {
  allowsCompletedPacketHistoricalFailSupersededDrift,
} from "../checks/refinement-check.mjs";

test("allows historical failed packet evidence when task board supersedes it to active recovery", () => {
  assert.equal(
    allowsCompletedPacketHistoricalFailSupersededDrift({
      groupLabel: "MATCHED_COMPLETED_PACKETS",
      rowBoardStatus: "FAIL",
      actualTaskBoardStatus: "SUPERSEDED",
      rowResolution: "EXPAND_IN_THIS_WP",
      rowArtifact: "WP-1-Calendar-Sync-Engine-v2",
      currentWpId: "WP-1-Calendar-Sync-Engine-v3",
      packetStatus: "Validated (FAIL)",
      activePacketId: "WP-1-Calendar-Sync-Engine-v3",
    }),
    true,
  );
});

test("rejects historical fail exception without active recovery targeting current WP", () => {
  assert.equal(
    allowsCompletedPacketHistoricalFailSupersededDrift({
      groupLabel: "MATCHED_COMPLETED_PACKETS",
      rowBoardStatus: "FAIL",
      actualTaskBoardStatus: "SUPERSEDED",
      rowResolution: "EXPAND_IN_THIS_WP",
      rowArtifact: "WP-1-Calendar-Sync-Engine-v2",
      currentWpId: "WP-1-Calendar-Sync-Engine-v3",
      packetStatus: "Validated (FAIL)",
      activePacketId: "WP-1-Calendar-Sync-Engine-v4",
    }),
    false,
  );
});

test("rejects non-fail packet statuses for the historical fail exception", () => {
  assert.equal(
    allowsCompletedPacketHistoricalFailSupersededDrift({
      groupLabel: "MATCHED_COMPLETED_PACKETS",
      rowBoardStatus: "FAIL",
      actualTaskBoardStatus: "SUPERSEDED",
      rowResolution: "EXPAND_IN_THIS_WP",
      rowArtifact: "WP-1-Calendar-Sync-Engine-v2",
      currentWpId: "WP-1-Calendar-Sync-Engine-v3",
      packetStatus: "Validated (PASS)",
      activePacketId: "WP-1-Calendar-Sync-Engine-v3",
    }),
    false,
  );
});

test("normalizes markdown-bold residue on packet status for historical fail exception", () => {
  assert.equal(
    allowsCompletedPacketHistoricalFailSupersededDrift({
      groupLabel: "MATCHED_COMPLETED_PACKETS",
      rowBoardStatus: "FAIL",
      actualTaskBoardStatus: "SUPERSEDED",
      rowResolution: "EXPAND_IN_THIS_WP",
      rowArtifact: "WP-1-Calendar-Sync-Engine-v2",
      currentWpId: "WP-1-Calendar-Sync-Engine-v3",
      packetStatus: "** Validated (FAIL)",
      activePacketId: "WP-1-Calendar-Sync-Engine-v3",
    }),
    true,
  );
});
