import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { communicationPathsForWp } from "../scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs } from "../scripts/lib/runtime-paths.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const scriptPath = path.join(repoRoot, ".GOV", "roles_shared", "scripts", "wp", "mt-board.mjs");

function runMtBoard(args) {
  return execFileSync(process.execPath, [scriptPath, ...args], {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function writeContract(absPath, value) {
  fs.mkdirSync(path.dirname(absPath), { recursive: true });
  fs.writeFileSync(absPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

test("mt-board claims and completes JSON microtask contracts", () => {
  const wpId = "WP-TEST-MT-BOARD-JSON-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  writeContract(path.join(packetDir, "packet.json"), {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
  });
  writeContract(path.join(packetDir, "MT-001.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-001",
    wp_id: wpId,
    title: "First JSON MT",
    owned_files: ["src/first.rs"],
    proof_commands: ["cargo test first"],
    depends_on_mts: [],
    lifecycle: { status: "PENDING", depends_on: [] },
    handoff: {},
  });
  writeContract(path.join(packetDir, "MT-002.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-002",
    wp_id: wpId,
    title: "Second JSON MT",
    owned_files: ["src/second.rs"],
    proof_commands: ["cargo test second"],
    depends_on_mts: ["MT-001"],
    lifecycle: { status: "PENDING", depends_on: ["MT-001"] },
    handoff: {},
  });

  try {
    assert.match(runMtBoard(["board", wpId]), /MT-001 \| PENDING/);
    assert.match(runMtBoard(["claim", wpId, "session-json"]), /Claimed MT-001/);
    let mt001 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-001.json"), "utf8"));
    assert.equal(mt001.lifecycle.status, "CLAIMED");
    assert.equal(mt001.handoff.coder_session, "session-json");

    assert.match(runMtBoard(["claim", wpId, "session-json"]), /No unclaimed microtasks/);
    assert.match(runMtBoard(["complete", wpId, "MT-001"]), /MT-001 marked completed/);
    mt001 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-001.json"), "utf8"));
    assert.equal(mt001.lifecycle.status, "COMPLETED");

    assert.match(runMtBoard(["claim", wpId, "session-json"]), /Claimed MT-002/);
    const mt002 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-002.json"), "utf8"));
    assert.equal(mt002.lifecycle.status, "CLAIMED");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("mt-board preserves blocked microtasks and skips them during claim", () => {
  const wpId = "WP-TEST-MT-BOARD-BLOCKED-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  writeContract(path.join(packetDir, "packet.json"), {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
  });
  writeContract(path.join(packetDir, "MT-001.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-001",
    wp_id: wpId,
    title: "Blocked JSON MT",
    owned_files: ["src/blocked.rs"],
    proof_commands: ["cargo test blocked"],
    depends_on_mts: [],
    lifecycle: { status: "BLOCKED", depends_on: [], blocker: { reason: "operator decision" } },
    handoff: {},
  });
  writeContract(path.join(packetDir, "MT-002.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-002",
    wp_id: wpId,
    title: "Available JSON MT",
    owned_files: ["src/available.rs"],
    proof_commands: ["cargo test available"],
    depends_on_mts: [],
    lifecycle: { status: "PENDING", depends_on: [] },
    handoff: {},
  });

  try {
    const board = runMtBoard(["board", wpId]);
    assert.match(board, /MT-001 \| BLOCKED/);
    assert.match(runMtBoard(["claim", wpId, "session-json"]), /Claimed MT-002/);
    const mt001 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-001.json"), "utf8"));
    const mt002 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-002.json"), "utf8"));
    assert.equal(mt001.lifecycle.status, "BLOCKED");
    assert.equal(mt001.handoff.coder_session, undefined);
    assert.equal(mt002.lifecycle.status, "CLAIMED");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("mt-board preserves managed-resource-proof blockers and skips them during claim", () => {
  const wpId = "WP-TEST-MT-BOARD-MANAGED-RESOURCE-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  writeContract(path.join(packetDir, "packet.json"), {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
  });
  writeContract(path.join(packetDir, "MT-001.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-001",
    wp_id: wpId,
    title: "Managed resource proof blocked JSON MT",
    owned_files: ["src/managed.rs"],
    proof_commands: ["cargo test managed"],
    depends_on_mts: [],
    lifecycle: {
      status: "NEEDS_MANAGED_RESOURCE_PROOF",
      depends_on: [],
      missing_resource: "REAL_CANDLE_CPU_MODEL_LOAD",
    },
    handoff: {},
  });
  writeContract(path.join(packetDir, "MT-002.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-002",
    wp_id: wpId,
    title: "Available JSON MT",
    owned_files: ["src/available.rs"],
    proof_commands: ["cargo test available"],
    depends_on_mts: [],
    lifecycle: { status: "PENDING", depends_on: [] },
    handoff: {},
  });

  try {
    const board = runMtBoard(["board", wpId]);
    assert.match(board, /MT-001 \| NEEDS_MANAGED_RESOURCE_PROOF/);
    assert.match(runMtBoard(["claim", wpId, "session-json"]), /Claimed MT-002/);
    const mt001 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-001.json"), "utf8"));
    const mt002 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-002.json"), "utf8"));
    assert.equal(mt001.lifecycle.status, "NEEDS_MANAGED_RESOURCE_PROOF");
    assert.equal(mt001.handoff.coder_session, undefined);
    assert.equal(mt002.lifecycle.status, "CLAIMED");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("mt-board treats open microtasks as visible and claimable", () => {
  const wpId = "WP-TEST-MT-BOARD-OPEN-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  writeContract(path.join(packetDir, "packet.json"), {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
  });
  writeContract(path.join(packetDir, "MT-001.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-001",
    wp_id: wpId,
    title: "Open JSON MT",
    owned_files: ["src/open.rs"],
    proof_commands: ["cargo test open"],
    depends_on_mts: [],
    lifecycle: { status: "OPEN", active: false, depends_on: [] },
    handoff: {},
  });

  try {
    const board = runMtBoard(["board", wpId]);
    assert.match(board, /MT-001 \| OPEN/);
    assert.match(runMtBoard(["claim", wpId, "session-json"]), /Claimed MT-001/);
    const mt001 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-001.json"), "utf8"));
    assert.equal(mt001.lifecycle.status, "CLAIMED");
    assert.equal(mt001.handoff.coder_session, "session-json");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("mt-board treats ready-for-validation microtasks as implemented dependency evidence", () => {
  const wpId = "WP-TEST-MT-BOARD-READY-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  writeContract(path.join(packetDir, "packet.json"), {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
  });
  writeContract(path.join(packetDir, "MT-001.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-001",
    wp_id: wpId,
    title: "Ready JSON MT",
    owned_files: ["src/ready.rs"],
    proof_commands: ["cargo test ready"],
    depends_on_mts: [],
    lifecycle: {
      status: "READY_FOR_VALIDATION",
      active: false,
      depends_on: [],
      ready_for_validation_at_utc: "2026-05-24T00:00:00Z",
    },
    handoff: { coder_session: "session-ready" },
  });
  writeContract(path.join(packetDir, "MT-002.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-002",
    wp_id: wpId,
    title: "Downstream JSON MT",
    owned_files: ["src/downstream.rs"],
    proof_commands: ["cargo test downstream"],
    depends_on_mts: ["MT-001"],
    lifecycle: { status: "PENDING", depends_on: ["MT-001"] },
    handoff: {},
  });

  try {
    const board = runMtBoard(["board", wpId]);
    assert.match(board, /MT-001 \| READY_FOR_VALIDATION/);
    assert.match(runMtBoard(["claim", wpId, "session-json"]), /Claimed MT-002/);
    const mt001 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-001.json"), "utf8"));
    const mt002 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-002.json"), "utf8"));
    assert.equal(mt001.lifecycle.status, "READY_FOR_VALIDATION");
    assert.equal(mt001.handoff.coder_session, "session-ready");
    assert.equal(mt002.lifecycle.status, "CLAIMED");
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});

test("mt-board requires the claimant's passing readiness receipt before READY_FOR_VALIDATION", () => {
  const wpId = "WP-TEST-MT-BOARD-READY-GATE-v1";
  const session = "session-ready-gate";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = repoPathAbs(communicationPathsForWp(wpId).dir);
  writeContract(path.join(packetDir, "packet.json"), {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
  });
  writeContract(path.join(packetDir, "MT-001.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-001",
    wp_id: wpId,
    title: "Ready-gated JSON MT",
    owned_files: ["src/ready.rs"],
    proof_commands: ["cargo test ready"],
    lifecycle: { status: "CLAIMED", active: true, claimed_by: session, completed_by: null },
    handoff: { coder_session: session },
  });

  try {
    assert.throws(
      () => runMtBoard(["ready", wpId, "MT-001", session]),
      /requires a latest PASS KB readiness receipt/,
    );
    fs.mkdirSync(commDir, { recursive: true });
    fs.writeFileSync(path.join(commDir, "KB_READY_CHECKLIST_RECEIPTS.jsonl"), `${JSON.stringify({
      schema_id: "hsk.kb_ready_checklist_receipt@1",
      wp_id: wpId,
      mt_id: "MT-001",
      actor_session: session,
      overall_verdict: "PASS",
    })}\n`, "utf8");
    assert.match(runMtBoard(["ready", wpId, "MT-001", session]), /marked ready for validation/);
    const mt001 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-001.json"), "utf8"));
    assert.equal(mt001.lifecycle.status, "READY_FOR_VALIDATION");
    assert.equal(mt001.lifecycle.active, false);
    assert.equal(mt001.lifecycle.completed_by, null);
    assert.equal(mt001.lifecycle.ready_for_validation_by, session);
    assert.match(mt001.lifecycle.ready_for_validation_at_utc, /^\d{4}-\d{2}-\d{2}T/);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
    fs.rmSync(commDir, { recursive: true, force: true });
  }
});

test("mt-board targets ready-for-dev MTs without collapsing versioned lifecycle states", () => {
  const wpId = "WP-TEST-MT-BOARD-TARGETED-v1";
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  writeContract(path.join(packetDir, "packet.json"), {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: wpId,
  });
  writeContract(path.join(packetDir, "MT-001.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-001",
    wp_id: wpId,
    title: "Versioned implemented MT",
    owned_files: ["src/implemented.rs"],
    proof_commands: ["cargo test implemented"],
    lifecycle: { status: "READY_FOR_VALIDATION_V4", active: false, depends_on: [] },
    handoff: { coder_session: "prior-session" },
  });
  writeContract(path.join(packetDir, "MT-002.json"), {
    schema_id: "hsk.microtask_contract@1",
    mt_id: "MT-002",
    wp_id: wpId,
    title: "Exact ready MT",
    owned_files: ["src/ready.rs"],
    proof_commands: ["cargo test ready"],
    lifecycle: { status: "READY_FOR_DEV", active: true, depends_on: ["MT-001"] },
    handoff: {},
  });

  try {
    const board = runMtBoard(["board", wpId]);
    assert.match(board, /MT-001 \| READY_FOR_VALIDATION/);
    assert.match(board, /MT-002 \| OPEN/);
    assert.match(runMtBoard(["claim", wpId, "session-targeted", "MT-002"]), /Claimed MT-002/);
    const mt002 = JSON.parse(fs.readFileSync(path.join(packetDir, "MT-002.json"), "utf8"));
    assert.equal(mt002.lifecycle.status, "CLAIMED");
    assert.equal(mt002.lifecycle.claimed_by, "session-targeted");
    assert.equal(mt002.handoff.coder_session, "session-targeted");
    assert.match(mt002.lifecycle.claimed_at_utc, /^\d{4}-\d{2}-\d{2}T/);
  } finally {
    fs.rmSync(packetDir, { recursive: true, force: true });
  }
});
