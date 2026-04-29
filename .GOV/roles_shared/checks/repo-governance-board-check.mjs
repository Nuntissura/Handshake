#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  REPO_GOVERNANCE_BOARD_PATH,
  REPO_GOVERNANCE_CHANGELOG_PATH,
  WP1_POSTMORTEM_GUIDE_PATH,
  validateRepoGovernanceBoard,
} from "../scripts/lib/repo-governance-board-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("repo-governance-board-check.mjs", { role: "SHARED" });

function repoRootFromHere() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function fail(message, details = []) {
  failWithMemory("repo-governance-board-check.mjs", message, { role: "SHARED", details });
}

const repoRoot = path.resolve(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || repoRootFromHere());
const boardAbs = path.resolve(repoRoot, REPO_GOVERNANCE_BOARD_PATH);
if (!fs.existsSync(boardAbs)) {
  fail("Repo governance refactor board is missing", [REPO_GOVERNANCE_BOARD_PATH]);
}

const changelogAbs = path.resolve(repoRoot, REPO_GOVERNANCE_CHANGELOG_PATH);
const guideAbs = path.resolve(repoRoot, WP1_POSTMORTEM_GUIDE_PATH);
const result = validateRepoGovernanceBoard({
  repoRoot,
  boardText: fs.readFileSync(boardAbs, "utf8"),
  changelogText: fs.existsSync(changelogAbs) ? fs.readFileSync(changelogAbs, "utf8") : "",
  guideText: fs.existsSync(guideAbs) ? fs.readFileSync(guideAbs, "utf8") : "",
});

if (!result.ok) {
  fail("Repo governance refactor board integrity violations found", result.errors);
}

console.log("repo-governance-board-check ok");
console.log(`- rows: ${result.summary.row_count}`);
console.log(`- rgf_rows: ${result.summary.rgf_count}`);
console.log(`- done: ${result.summary.done_count}`);
console.log(`- planned: ${result.summary.planned_count}`);
if (result.warnings.length > 0) {
  console.log(`- warnings: ${result.warnings.join(" | ")}`);
}

