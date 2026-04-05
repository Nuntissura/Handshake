import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { deriveWpMicrotaskPlan, listDeclaredWpMicrotasks } from "../scripts/lib/wp-microtask-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

function writeMicrotask(packetDir, wpId, mtId, clause, codeSurfaces) {
  fs.writeFileSync(
    path.join(packetDir, `${mtId}.md`),
    [
      `# ${mtId}: ${clause}`,
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      `- MT_ID: ${mtId}`,
      `- CLAUSE: ${clause}`,
      `- CODE_SURFACES: ${codeSurfaces.join("; ")}`,
      "- EXPECTED_TESTS: cargo test demo",
      "- DEPENDS_ON: NONE",
    ].join("\n"),
    "utf8",
  );
}

test("deriveWpMicrotaskPlan keeps kickoff-reviewed microtasks as the active execution budget", () => {
  const wpId = "WP-TEST-MICROTASK-PLAN-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(packetDir, wpId, "MT-001", "Bootstrap scope [CX-MT-001]", ["src/bootstrap.rs"]);
  writeMicrotask(packetDir, wpId, "MT-002", "Handoff scope [CX-MT-002]", ["src/handoff.rs"]);

  try {
    const microtasks = listDeclaredWpMicrotasks(wpId);
    const plan = deriveWpMicrotaskPlan({
      wpId,
      microtasks,
      receipts: [
        {
          timestamp_utc: "2026-04-05T10:00:00Z",
          actor_role: "CODER",
          receipt_kind: "CODER_INTENT",
          correlation_id: "intent-1",
          microtask_contract: {
            scope_ref: "MT-001",
          },
        },
      ],
      runtimeStatus: {
        open_review_items: [
          {
            correlation_id: "intent-1",
            receipt_kind: "VALIDATOR_QUERY",
            opened_by_role: "WP_VALIDATOR",
            updated_at: "2026-04-05T10:02:00Z",
            microtask_contract: {
              scope_ref: "MT-001",
            },
          },
        ],
      },
    });

    assert.equal(plan.declared_count, 2);
    assert.equal(plan.active_microtask?.mt_id, "MT-001");
    assert.equal(plan.active_microtask?.state, "IN_REVIEW");
    assert.equal(plan.previous_microtask, null);
    assert.equal(plan.suggested_next_microtask?.mt_id, "MT-002");
    assert.equal(plan.items[1].state, "DECLARED");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("deriveWpMicrotaskPlan advances execution to the next declared microtask after overlap review opens", () => {
  const wpId = "WP-TEST-MICROTASK-OVERLAP-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(packetDir, wpId, "MT-001", "Bootstrap scope [CX-MT-101]", ["src/bootstrap.rs"]);
  writeMicrotask(packetDir, wpId, "MT-002", "Follow-on scope [CX-MT-102]", ["src/follow_on.rs"]);

  try {
    const plan = deriveWpMicrotaskPlan({
      wpId,
      receipts: [
        {
          timestamp_utc: "2026-04-05T10:00:00Z",
          actor_role: "CODER",
          receipt_kind: "REVIEW_REQUEST",
          correlation_id: "review-1",
          microtask_contract: {
            scope_ref: "MT-001",
            review_mode: "OVERLAP",
          },
        },
      ],
      runtimeStatus: {
        open_review_items: [
          {
            correlation_id: "review-1",
            receipt_kind: "REVIEW_REQUEST",
            opened_by_role: "CODER",
            updated_at: "2026-04-05T10:02:00Z",
            microtask_contract: {
              scope_ref: "MT-001",
              review_mode: "OVERLAP",
            },
          },
        ],
      },
    });

    assert.equal(plan.active_microtask?.mt_id, "MT-002");
    assert.equal(plan.active_microtask?.state, "DECLARED");
    assert.equal(plan.previous_microtask?.mt_id, "MT-001");
    assert.equal(plan.previous_microtask?.state, "IN_REVIEW");
    assert.equal(plan.suggested_next_microtask, null);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("deriveWpMicrotaskPlan promotes validator repair outcomes above later declared work", () => {
  const wpId = "WP-TEST-MICROTASK-REPAIR-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(packetDir, wpId, "MT-001", "Repair scope [CX-MT-003]", ["src/repair.rs"]);
  writeMicrotask(packetDir, wpId, "MT-002", "Later scope [CX-MT-004]", ["src/later.rs"]);

  try {
    const plan = deriveWpMicrotaskPlan({
      wpId,
      receipts: [
        {
          timestamp_utc: "2026-04-05T10:00:00Z",
          actor_role: "CODER",
          receipt_kind: "REVIEW_REQUEST",
          correlation_id: "review-1",
          microtask_contract: {
            scope_ref: "MT-001",
          },
        },
        {
          timestamp_utc: "2026-04-05T10:05:00Z",
          actor_role: "WP_VALIDATOR",
          target_role: "CODER",
          receipt_kind: "VALIDATOR_REVIEW",
          correlation_id: "review-1",
          summary: "Repair required before re-handoff.",
          microtask_contract: {
            scope_ref: "MT-001",
            review_outcome: "REPAIR_REQUIRED",
          },
        },
      ],
      runtimeStatus: {},
    });

    assert.equal(plan.active_microtask?.mt_id, "MT-001");
    assert.equal(plan.active_microtask?.state, "REPAIR_REQUIRED");
    assert.equal(plan.previous_microtask, null);
    assert.equal(plan.suggested_next_microtask?.mt_id, "MT-001");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});
