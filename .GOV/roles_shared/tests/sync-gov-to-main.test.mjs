import assert from "node:assert/strict";
import test from "node:test";
import { classifyMainWorktreeGovSyncStatus } from "../scripts/topology/sync-gov-to-main.mjs";

test("sync-gov-to-main allows unrelated unstaged drift outside .GOV", () => {
  const classification = classifyMainWorktreeGovSyncStatus([
    " M AGENTS.md",
    "?? scratch-notes.txt",
  ].join("\n"));

  assert.equal(classification.govEntries.length, 0);
  assert.equal(classification.stagedOutsideGovEntries.length, 0);
  assert.deepEqual(
    classification.unstagedOutsideGovEntries.map((entry) => entry.line),
    [" M AGENTS.md", "?? scratch-notes.txt"],
  );
});

test("sync-gov-to-main still blocks .GOV overlap and staged non-governance changes", () => {
  const classification = classifyMainWorktreeGovSyncStatus([
    " M .GOV/roles_shared/checks/phase-check.mjs",
    "M  README.md",
    "R  docs/old.md -> .GOV/docs/new.md",
  ].join("\n"));

  assert.deepEqual(
    classification.govEntries.map((entry) => entry.line),
    [" M .GOV/roles_shared/checks/phase-check.mjs", "R  docs/old.md -> .GOV/docs/new.md"],
  );
  assert.deepEqual(
    classification.stagedOutsideGovEntries.map((entry) => entry.line),
    ["M  README.md"],
  );
});
