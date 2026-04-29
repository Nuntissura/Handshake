import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { validateRepoGovernanceBoard } from "../scripts/lib/repo-governance-board-lib.mjs";

function boardWithRows(rows, { status = "**Status:** `RGF-255` through `RGF-256` are planned." } = {}) {
  return [
    "# Repo Governance Refactor Task Board",
    "",
    status,
    "",
    "## Post-Refactor Follow-On Board",
    "",
    "| ID | Status | Workstream | Depends On | Evidence | Primary Surfaces | Exit Signal |",
    "|---|---|---|---|---|---|---|",
    ...rows,
    "",
    "## Proposed Next Sequence",
    "",
    "1. `RGF-255`",
  ].join("\n");
}

test("repo governance board check rejects duplicate row ids", () => {
  const board = boardWithRows([
    "| RGF-255 | PLANNED | A | - | audit | `x` | signal |",
    "| RGF-255 | PLANNED | B | - | audit | `x` | signal |",
    "| RGF-256 | PLANNED | C | RGF-255 | audit | `x` | signal |",
  ]);
  const result = validateRepoGovernanceBoard({ boardText: board });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /duplicate row id RGF-255/);
});

test("repo governance board check rejects unknown sequence references", () => {
  const board = [
    boardWithRows([
      "| RGF-255 | PLANNED | A | - | audit | `x` | signal |",
      "| RGF-256 | PLANNED | B | RGF-255 | audit | `x` | signal |",
    ]),
    "2. `RGF-999`",
  ].join("\n");
  const result = validateRepoGovernanceBoard({ boardText: board });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /unknown RGF-999/);
});

test("repo governance board check rejects missing implementation brief files", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-board-check-"));
  try {
    const board = [
      boardWithRows([
        "| RGF-255 | PLANNED | A | - | audit | `x` | signal |",
        "| RGF-256 | PLANNED | B | RGF-255 | audit | `x` | signal |",
      ]),
      "",
      "**Implementation briefs:** `.GOV/roles_shared/records/MISSING.md`",
    ].join("\n");
    const result = validateRepoGovernanceBoard({ repoRoot, boardText: board });
    assert.equal(result.ok, false);
    assert.match(result.errors.join("\n"), /referenced file does not exist/);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});

test("repo governance board check accepts a clean planned tranche", () => {
  const board = boardWithRows([
    "| RGF-255 | PLANNED | Compact WP Truth Bundle | - | audit | `roles_shared/scripts/lib/wp-truth-bundle-lib.mjs` | one command emits truth |",
    "| RGF-256 | PLANNED | Executable Packet Acceptance Matrix | RGF-255 | audit | `packet-closure-monitor-lib.mjs` | packet rows close |",
  ]);
  const guide = [
    "| ID | Status | Workstream | Depends On | Evidence | Primary Surfaces | Exit Signal |",
    "|---|---|---|---|---|---|---|",
    "| RGF-255 | PLANNED | Compact WP Truth Bundle | - | audit | `roles_shared/scripts/lib/wp-truth-bundle-lib.mjs` | one command emits truth |",
    "| RGF-256 | PLANNED | Executable Packet Acceptance Matrix | RGF-255 | audit | `packet-closure-monitor-lib.mjs` | packet rows close |",
  ].join("\n");
  const result = validateRepoGovernanceBoard({ boardText: board, guideText: guide });
  assert.equal(result.ok, true);
});

