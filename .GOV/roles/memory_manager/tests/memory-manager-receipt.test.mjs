import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { appendMemoryManagerReceipt } from "../scripts/memory-manager-receipt.mjs";
import { communicationPathsForWp } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { checkNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";

function readJsonl(absPath) {
  return fs.readFileSync(absPath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

test("Memory Manager receipt append writes packetless receipts and notifies orchestrator", () => {
  const wpId = "WP-MEMORY-HYGIENE_TEST-RECEIPT";
  const paths = communicationPathsForWp(wpId);
  fs.rmSync(repoPathAbs(paths.dir), { recursive: true, force: true });

  try {
    const result = appendMemoryManagerReceipt({
      wpId,
      actorSession: "MEMORY_MANAGER:test-session",
      receiptKind: "MEMORY_PROPOSAL",
      summary: "Promote repeated ACP broker recovery pattern into a governed recovery rule.",
      backupRef: ".GOV/roles/memory_manager/proposals/test-proposal.md",
      correlationId: "mm-test-proposal-1",
      skipMemoryExtraction: true,
    });

    const receipts = readJsonl(repoPathAbs(result.scaffold.receiptsFile));
    assert.equal(receipts.length, 1);
    assert.equal(receipts[0].actor_role, "MEMORY_MANAGER");
    assert.equal(receipts[0].receipt_kind, "MEMORY_PROPOSAL");
    assert.equal(receipts[0].target_role, "ORCHESTRATOR");

    const notifications = checkNotifications({
      wpId,
      role: "ORCHESTRATOR",
    }, { assumeTransactionLock: true });
    assert.equal(notifications.pendingCount, 1);
    assert.equal(notifications.notifications[0].source_role, "MEMORY_MANAGER");
    assert.match(notifications.notifications[0].summary, /MEMORY_PROPOSAL:/);
  } finally {
    fs.rmSync(repoPathAbs(paths.dir), { recursive: true, force: true });
  }
});
