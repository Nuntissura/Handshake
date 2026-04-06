#!/usr/bin/env node
/**
 * RGF-102: MT Task Board CLI.
 *
 * Commands:
 *   board  <WP_ID>                          — show task board
 *   claim  <WP_ID> <MT_ID> <SESSION_KEY>    — claim an MT
 *   complete <WP_ID> <MT_ID>                — mark MT completed
 *   populate <WP_ID>                        — populate board from packet MTs
 */

import { openWpCommDb, claimNextMt, completeMt, formatMtBoard, populateMtTasks, getMtTasks } from "../lib/wp-comm-sqlite.mjs";
import { listDeclaredWpMicrotasks } from "../lib/wp-communications-lib.mjs";

const command = String(process.argv[2] || "").trim().toLowerCase();
const wpId = String(process.argv[3] || "").trim();

if (!command || !wpId) {
  console.error("Usage:");
  console.error("  node mt-board.mjs board <WP_ID>");
  console.error("  node mt-board.mjs claim <WP_ID> <SESSION_KEY>");
  console.error("  node mt-board.mjs complete <WP_ID> <MT_ID>");
  console.error("  node mt-board.mjs populate <WP_ID>");
  process.exit(1);
}

const { db } = openWpCommDb(wpId);

if (command === "board") {
  console.log(formatMtBoard(db, wpId));

} else if (command === "populate") {
  try {
    const declared = listDeclaredWpMicrotasks(wpId);
    if (declared.length === 0) {
      console.log(`[MT_BOARD] No declared microtasks found for ${wpId}`);
      process.exit(0);
    }
    const mts = declared.map((mt) => ({
      mtId: mt.mtId || mt.id || mt,
      description: mt.title || mt.description || "",
      complexityTier: mt.complexityTier || "MEDIUM",
    }));
    populateMtTasks(db, wpId, mts);
    console.log(`[MT_BOARD] Populated ${mts.length} microtasks for ${wpId}`);
    console.log(formatMtBoard(db, wpId));
  } catch (e) {
    console.error(`[MT_BOARD] Failed to populate: ${e.message}`);
    process.exit(1);
  }

} else if (command === "claim") {
  const sessionKey = String(process.argv[4] || "").trim();
  if (!sessionKey) {
    console.error("Usage: node mt-board.mjs claim <WP_ID> <SESSION_KEY>");
    process.exit(1);
  }
  const claimed = claimNextMt(db, wpId, sessionKey);
  if (!claimed) {
    console.log(`[MT_BOARD] No unclaimed microtasks available for ${wpId}`);
  } else {
    console.log(`[MT_BOARD] Claimed ${claimed.mtId} for ${sessionKey}`);
  }

} else if (command === "complete") {
  const mtId = String(process.argv[4] || "").trim();
  if (!mtId) {
    console.error("Usage: node mt-board.mjs complete <WP_ID> <MT_ID>");
    process.exit(1);
  }
  const result = completeMt(db, wpId, mtId);
  if (result.changes === 0) {
    console.log(`[MT_BOARD] ${mtId} is not in 'claimed' state — cannot complete`);
  } else {
    console.log(`[MT_BOARD] ${mtId} marked completed`);
  }

} else {
  console.error(`Unknown command: ${command}`);
  process.exit(1);
}

db.close();
