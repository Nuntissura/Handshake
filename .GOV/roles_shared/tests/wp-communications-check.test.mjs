import assert from "node:assert/strict";
import test from "node:test";

import { isPacketlessSyntheticWp } from "../checks/wp-communications-check.mjs";

test("packetless synthetic Memory Manager lanes are recognized by wp communications checks", () => {
  assert.equal(isPacketlessSyntheticWp("WP-MEMORY-HYGIENE_2026-04-09T2232Z"), true);
  assert.equal(isPacketlessSyntheticWp("WP-1-Example-v1"), false);
});
