import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  deriveWpMicrotaskPlan,
  listDeclaredWpMicrotasks,
  summarizeMicrotaskFileTargetBudget,
} from "../scripts/lib/wp-microtask-lib.mjs";

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

test("listDeclaredWpMicrotasks parses hydrated backtick lists for code surfaces", () => {
  const wpId = "WP-TEST-MICROTASK-HYDRATED-LISTS-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  fs.writeFileSync(
    path.join(packetDir, "MT-001.md"),
    [
      "# MT-001: Hydrated list parsing",
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- MT_ID: MT-001",
      "- CLAUSE: Hydrated parser scope [CX-MT-HYDRATED]",
      "- CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/flight_recorder/mod.rs`",
      "- EXPECTED_TESTS: `cargo test alpha --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test beta --manifest-path src/backend/handshake_core/Cargo.toml`",
      "- DEPENDS_ON: NONE",
    ].join("\n"),
    "utf8",
  );

  try {
    const microtasks = listDeclaredWpMicrotasks(wpId);
    assert.equal(microtasks.length, 1);
    assert.deepEqual(microtasks[0].codeSurfaces, [
      "src/backend/handshake_core/src/workflows.rs",
      "src/backend/handshake_core/src/flight_recorder/mod.rs",
    ]);
    assert.deepEqual(microtasks[0].expectedTests, [
      "cargo test alpha --manifest-path src/backend/handshake_core/Cargo.toml",
      "cargo test beta --manifest-path src/backend/handshake_core/Cargo.toml",
    ]);

    const budget = summarizeMicrotaskFileTargetBudget(
      ["src/backend/handshake_core/src/flight_recorder/mod.rs"],
      microtasks[0],
    );
    assert.equal(budget.ok, true);
    assert.deepEqual(budget.outOfBudgetTargets, []);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("deriveWpMicrotaskPlan treats MT STEER review responses as repair required", () => {
  const wpId = "WP-TEST-MICROTASK-STEER-SUMMARY-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(packetDir, wpId, "MT-001", "Steer summary scope [CX-MT-STEER]", ["src/steer.rs"]);

  try {
    const microtasks = listDeclaredWpMicrotasks(wpId);
    const plan = deriveWpMicrotaskPlan({
      wpId,
      microtasks,
      receipts: [
        {
          timestamp_utc: "2026-04-05T10:00:00Z",
          actor_role: "CODER",
          receipt_kind: "REVIEW_REQUEST",
          correlation_id: "mt-001-review",
          microtask_contract: {
            scope_ref: "MT-001",
          },
        },
        {
          timestamp_utc: "2026-04-05T10:05:00Z",
          actor_role: "WP_VALIDATOR",
          receipt_kind: "REVIEW_RESPONSE",
          correlation_id: "mt-001-review",
          ack_for: "mt-001-review",
          summary: "MT-001 STEER: helper is not wired into production path. Repair by integrating it.",
          microtask_contract: {
            scope_ref: "MT-001",
          },
        },
      ],
    });

    assert.equal(plan.active_microtask?.mt_id, "MT-001");
    assert.equal(plan.active_microtask?.state, "REPAIR_REQUIRED");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("deriveWpMicrotaskPlan ignores status and thread prose when deriving microtask state", () => {
  const wpId = "WP-TEST-MICROTASK-NON-REVIEW-PROSE-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(packetDir, wpId, "MT-001", "Earlier scope [CX-MT-401]", ["src/earlier.rs"]);
  writeMicrotask(packetDir, wpId, "MT-002", "Final scope [CX-MT-402]", ["src/final.rs"]);

  try {
    const microtasks = listDeclaredWpMicrotasks(wpId);
    const plan = deriveWpMicrotaskPlan({
      wpId,
      microtasks,
      receipts: [
        {
          timestamp_utc: "2026-04-05T10:00:00Z",
          actor_role: "CODER",
          target_role: "WP_VALIDATOR",
          receipt_kind: "REVIEW_REQUEST",
          correlation_id: "mt-001-review",
          microtask_contract: {
            scope_ref: "MT-001",
          },
        },
        {
          timestamp_utc: "2026-04-05T10:05:00Z",
          actor_role: "WP_VALIDATOR",
          target_role: "CODER",
          receipt_kind: "REVIEW_RESPONSE",
          correlation_id: "mt-001-review",
          ack_for: "mt-001-review",
          summary: "MT-001 PASS: approved for final review.",
          microtask_contract: {
            scope_ref: "MT-001",
            review_outcome: "APPROVED_FOR_FINAL_REVIEW",
          },
        },
        {
          timestamp_utc: "2026-04-05T10:10:00Z",
          actor_role: "WP_VALIDATOR",
          target_role: "ORCHESTRATOR",
          receipt_kind: "THREAD_MESSAGE",
          correlation_id: "final-candidate-fail",
          summary: "FINAL_CANDIDATE_FAIL: MT-001..MT-002 were reviewed, but an integration-only issue remains.",
        },
        {
          timestamp_utc: "2026-04-05T10:11:00Z",
          actor_role: "CODER",
          target_role: "ORCHESTRATOR",
          receipt_kind: "STATUS",
          summary: "Status update mentions MT-001 and MT-002 while repair work proceeds elsewhere.",
        },
        {
          timestamp_utc: "2026-04-05T10:20:00Z",
          actor_role: "CODER",
          target_role: "WP_VALIDATOR",
          receipt_kind: "REVIEW_REQUEST",
          correlation_id: "mt-002-review",
          microtask_contract: {
            scope_ref: "MT-002",
          },
        },
        {
          timestamp_utc: "2026-04-05T10:25:00Z",
          actor_role: "WP_VALIDATOR",
          target_role: "CODER",
          receipt_kind: "REVIEW_RESPONSE",
          correlation_id: "mt-002-review",
          ack_for: "mt-002-review",
          summary: "MT-002 PASS: final candidate pass.",
          microtask_contract: {
            scope_ref: "MT-002",
          },
        },
      ],
    });

    assert.equal(plan.active_microtask, null);
    assert.equal(plan.previous_microtask?.mt_id, "MT-002");
    assert.deepEqual(plan.items.map((entry) => [entry.mt_id, entry.state, entry.state_reason]), [
      ["MT-001", "CLEARED", "receipt:REVIEW_RESPONSE"],
      ["MT-002", "CLEARED", "receipt:REVIEW_RESPONSE"],
    ]);
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

test("deriveWpMicrotaskPlan recovers overlap routing from MT packet_row_ref and correlation when microtask_contract is missing", () => {
  const wpId = "WP-TEST-MICROTASK-FALLBACK-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(packetDir, wpId, "MT-001", "Boundary scope [CX-MT-301]", ["src/boundary.rs"]);
  writeMicrotask(packetDir, wpId, "MT-002", "Next scope [CX-MT-302]", ["src/next.rs"]);

  try {
    const plan = deriveWpMicrotaskPlan({
      wpId,
      receipts: [
        {
          timestamp_utc: "2026-04-05T10:00:00Z",
          actor_role: "CODER",
          target_role: "WP_VALIDATOR",
          receipt_kind: "REVIEW_REQUEST",
          correlation_id: "review-1",
          packet_row_ref: "MT-001",
          summary: "MT-001 review_mode=OVERLAP: review the completed slice while I continue.",
        },
        {
          timestamp_utc: "2026-04-05T10:01:00Z",
          actor_role: "WP_VALIDATOR",
          target_role: "CODER",
          receipt_kind: "REVIEW_RESPONSE",
          correlation_id: "review-1",
          ack_for: "review-1",
          summary: "PASS. proceed.",
        },
      ],
      runtimeStatus: {
        open_review_items: [
          {
            correlation_id: "review-1",
            receipt_kind: "REVIEW_REQUEST",
            opened_by_role: "CODER",
            target_role: "WP_VALIDATOR",
            packet_row_ref: "MT-001",
            summary: "MT-001 review_mode=OVERLAP: review the completed slice while I continue.",
            opened_at: "2026-04-05T10:00:00Z",
            updated_at: "2026-04-05T10:00:00Z",
          },
        ],
      },
    });

    assert.equal(plan.previous_microtask?.mt_id, "MT-001");
    assert.equal(plan.previous_microtask?.review_mode, "OVERLAP");
    assert.equal(plan.active_microtask?.mt_id, "MT-002");
    assert.equal(plan.active_microtask?.state, "DECLARED");
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

test("deriveWpMicrotaskPlan keeps the current microtask active when overlap review on the previous microtask fails", () => {
  const wpId = "WP-TEST-MICROTASK-DEFERRED-REPAIR-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  fs.mkdirSync(packetDir, { recursive: true });
  fs.writeFileSync(path.join(packetDir, "packet.md"), `- WP_ID: ${wpId}\n`, "utf8");
  writeMicrotask(packetDir, wpId, "MT-001", "Review-first scope [CX-MT-201]", ["src/review_first.rs"]);
  writeMicrotask(packetDir, wpId, "MT-002", "Current active scope [CX-MT-202]", ["src/current_active.rs"]);

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
        {
          timestamp_utc: "2026-04-05T10:01:00Z",
          actor_role: "CODER",
          receipt_kind: "CODER_INTENT",
          correlation_id: "intent-2",
          microtask_contract: {
            scope_ref: "MT-002",
          },
        },
        {
          timestamp_utc: "2026-04-05T10:02:00Z",
          actor_role: "WP_VALIDATOR",
          target_role: "CODER",
          receipt_kind: "VALIDATOR_REVIEW",
          correlation_id: "review-1",
          summary: "Repair required on MT-001 before wider closure.",
          microtask_contract: {
            scope_ref: "MT-001",
            review_mode: "OVERLAP",
            review_outcome: "REPAIR_REQUIRED",
          },
        },
      ],
      runtimeStatus: {},
    });

    assert.equal(plan.active_microtask?.mt_id, "MT-002");
    assert.equal(plan.active_microtask?.state, "ACTIVE");
    assert.equal(plan.previous_microtask?.mt_id, "MT-001");
    assert.equal(plan.previous_microtask?.state, "REPAIR_REQUIRED");
    assert.equal(plan.attention_microtask?.mt_id, "MT-001");
    assert.equal(plan.suggested_next_microtask?.mt_id, "MT-001");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});
