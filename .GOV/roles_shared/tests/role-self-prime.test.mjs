import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const selfPrimeScript = path.resolve(__dirname, "../scripts/session/role-self-prime.mjs");

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function writeText(filePath, text = "") {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, text, "utf8");
}

function appendJsonl(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, "utf8");
}

function sanitizePathSegment(value = "") {
  return String(value || "")
    .trim()
    .replace(/[^A-Za-z0-9._-]+/g, "_")
    .replace(/^_+|_+$/g, "")
    || "session";
}

function fixtureSessionEventsFile(fixture, sessionId) {
  return path.join(
    fixture.govRuntimeRoot,
    "roles_shared",
    "WP_SESSIONS",
    sanitizePathSegment(fixture.wpId),
    sanitizePathSegment(sessionId),
    "events.jsonl",
  );
}

function createFixture({
  workflowLane = "ORCHESTRATOR_MANAGED",
  nextExpectedActor = "CODER",
  nextExpectedSession = "CODER:WP-TEST-SELF-PRIME-v1",
  waitingOn = "CODER_INTENT",
  waitingOnSession = "CODER:WP-TEST-SELF-PRIME-v1",
} = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-role-self-prime-"));
  const govRoot = path.join(root, ".GOV");
  const govRuntimeRoot = path.join(root, "gov_runtime");
  const wpId = "WP-TEST-SELF-PRIME-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const notificationsPath = path.join(commDir, "NOTIFICATIONS.jsonl");
  const cursorPath = path.join(commDir, "NOTIFICATION_CURSOR.json");
  const registryPath = path.join(govRuntimeRoot, "roles_shared", "ROLE_SESSION_REGISTRY.json");

  writeText(packetPath, [
    `- WP_ID: ${wpId}`,
    "- **Status:** In Progress",
    `- WORKFLOW_LANE: ${workflowLane}`,
    "- PACKET_FORMAT_VERSION: 2026-03-22",
    "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1",
    "- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING",
    `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimeStatusPath)}`,
    `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
  ].join("\n"));

  writeText(path.join(govRoot, "task_packets", wpId, "MT-001.md"), [
    "# MT-001: Self-prime fixture [CX-SELF-001]",
    "",
    "## METADATA",
    `- WP_ID: ${wpId}`,
    "- MT_ID: MT-001",
    "- CLAUSE: Self-prime fixture [CX-SELF-001]",
    "- CODE_SURFACES: src/example.rs",
    "- EXPECTED_TESTS: cargo test self_prime_fixture -- --exact",
    "- DEPENDS_ON: NONE",
  ].join("\n"));

  writeText(runtimeStatusPath, `${JSON.stringify({
    schema_version: "wp_runtime_status@1",
    wp_id: wpId,
    task_packet: normalizePath(packetPath),
    communication_dir: normalizePath(commDir),
    thread_file: normalizePath(path.join(commDir, "THREAD.md")),
    runtime_status_file: normalizePath(runtimeStatusPath),
    receipts_file: normalizePath(receiptsPath),
    workflow_lane: workflowLane,
    runtime_status: "working",
    current_phase: "IMPLEMENTATION",
    current_task_board_status: "IN_PROGRESS",
    current_milestone: "MICROTASK",
    next_expected_actor: nextExpectedActor,
    next_expected_session: nextExpectedSession,
    waiting_on: waitingOn,
    waiting_on_session: waitingOnSession,
    active_role_sessions: [
      { role: "CODER", session_id: "CODER:WP-TEST-SELF-PRIME-v1", state: "working", last_heartbeat_at: "2099-01-01T10:00:00Z" },
    ],
    open_review_items: [],
    current_coder_revision_cycle: 0,
    current_validator_review_cycle: 0,
    current_relay_escalation_cycle: 0,
  }, null, 2)}\n`);
  writeText(receiptsPath, "");
  writeText(notificationsPath, "");
  writeText(cursorPath, `${JSON.stringify({ schema_version: "wp_notification_cursor@1", cursors: {} }, null, 2)}\n`);
  writeText(path.join(commDir, "THREAD.md"), "# thread\n");
  writeText(registryPath, `${JSON.stringify({ sessions: [] }, null, 2)}\n`);

  return { root, govRoot, govRuntimeRoot, wpId, registryPath };
}

function runSelfPrime(fixture, role, extraArgs = []) {
  return spawnSync(process.execPath, [
    selfPrimeScript,
    "--role", role,
    "--wp-id", fixture.wpId,
    "--mt-id", "MT-001",
    "--session-id", `${role}:${fixture.wpId}`,
    ...extraArgs,
  ], {
    cwd: repoRoot,
    env: {
      ...process.env,
      HANDSHAKE_GOV_ROOT: fixture.govRoot,
      HANDSHAKE_GOV_RUNTIME_ROOT: fixture.govRuntimeRoot,
    },
    encoding: "utf8",
  });
}

test("role-self-prime builds effective prompts for governed implementation roles", () => {
  const fixture = createFixture();
  for (const role of ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]) {
    const result = runSelfPrime(fixture, role);
    assert.equal(result.status, 0, `${role}\nSTDOUT:\n${result.stdout}\nSTDERR:\n${result.stderr}`);
    assert.match(result.stdout, /ROLE_SELF_PRIME \[RGF-246\]/);
    assert.match(result.stdout, new RegExp(`- ROLE: ${role}`));
    assert.match(result.stdout, /SOURCE_PRIORITY: terminal_closeout_record -> packet_projection -> mt_board -> runtime_status -> repomem\/governance_memory/);
    assert.match(result.stdout, /ACTIVE_LANE_BRIEF \[CX-LANE-001\]/);
    assert.match(result.stdout, /ROLE_SELF_PRIME_EFFECTIVE_PROMPT:/);
    assert.match(result.stdout, new RegExp(`ROLE LOCK: You are the ${role}`));
    assert.match(result.stdout, /SESSION_OPEN \(MANDATORY\): Before any governed mutation/i);
  }
});

test("role-self-prime builds effective prompts for classic manual-relay roles", () => {
  const fixture = createFixture({
    workflowLane: "MANUAL_RELAY",
    nextExpectedActor: "VALIDATOR",
    nextExpectedSession: "VALIDATOR:WP-TEST-SELF-PRIME-v1",
    waitingOn: "VALIDATOR_REVIEW",
    waitingOnSession: "VALIDATOR:WP-TEST-SELF-PRIME-v1",
  });
  for (const role of ["CLASSIC_ORCHESTRATOR", "VALIDATOR"]) {
    const result = runSelfPrime(fixture, role);
    assert.equal(result.status, 0, `${role}\nSTDOUT:\n${result.stdout}\nSTDERR:\n${result.stderr}`);
    assert.match(result.stdout, /ROLE_SELF_PRIME \[RGF-246\]/);
    assert.match(result.stdout, new RegExp(`- ROLE: ${role}`));
    assert.match(result.stdout, /lane=MANUAL_RELAY/);
    assert.match(result.stdout, /ROLE_SELF_PRIME_EFFECTIVE_PROMPT:/);
    assert.match(result.stdout, new RegExp(`ROLE LOCK: You are the ${role}`));
    assert.match(result.stdout, /SESSION_OPEN \(MANDATORY\): Before any governed mutation/i);
    assert.doesNotMatch(result.stdout, new RegExp(`active-lane-brief ${role}`));
  }
});

test("role-self-prime includes same-role predecessor summary when available", () => {
  const fixture = createFixture();
  const previousSessionId = `CODER:${fixture.wpId}:previous`;
  const currentSessionId = `CODER:${fixture.wpId}`;
  appendJsonl(fixtureSessionEventsFile(fixture, previousSessionId), {
    schema_id: "hsk.session_event@1",
    schema_version: "session_event_v1",
    timestamp: "2026-04-27T10:00:00Z",
    wp_id: fixture.wpId,
    role: "CODER",
    session_id: previousSessionId,
    event_type: "tool_call",
    tool_name: "prior_compile_check",
    result_class: "PASS",
  });
  writeText(fixture.registryPath, `${JSON.stringify({
    sessions: [
      {
        session_key: previousSessionId,
        session_id: previousSessionId,
        wp_id: fixture.wpId,
        role: "CODER",
        last_command_completed_at: "2026-04-27T10:00:00Z",
      },
      {
        session_key: currentSessionId,
        session_id: currentSessionId,
        wp_id: fixture.wpId,
        role: "CODER",
        last_event_at: "2026-04-27T11:00:00Z",
      },
    ],
  }, null, 2)}\n`);

  const result = runSelfPrime(fixture, "CODER");

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /<predecessor-summary/);
  assert.match(result.stdout, /PREDECESSOR_SESSION_ID: CODER:WP-TEST-SELF-PRIME-v1:previous/);
  assert.match(result.stdout, /prior_compile_check/);
});

test("role-self-prime includes predecessor summary for classical validator", () => {
  const fixture = createFixture({
    workflowLane: "MANUAL_RELAY",
    nextExpectedActor: "VALIDATOR",
    nextExpectedSession: "VALIDATOR:WP-TEST-SELF-PRIME-v1",
    waitingOn: "VALIDATOR_REVIEW",
    waitingOnSession: "VALIDATOR:WP-TEST-SELF-PRIME-v1",
  });
  const previousSessionId = `VALIDATOR:${fixture.wpId}:previous`;
  const currentSessionId = `VALIDATOR:${fixture.wpId}`;
  appendJsonl(fixtureSessionEventsFile(fixture, previousSessionId), {
    schema_id: "hsk.session_event@1",
    schema_version: "session_event_v1",
    timestamp: "2026-04-27T10:00:00Z",
    wp_id: fixture.wpId,
    role: "VALIDATOR",
    session_id: previousSessionId,
    event_type: "tool_call",
    tool_name: "prior_manual_validation_check",
    result_class: "PASS",
  });
  writeText(fixture.registryPath, `${JSON.stringify({
    sessions: [
      {
        session_key: previousSessionId,
        session_id: previousSessionId,
        wp_id: fixture.wpId,
        role: "VALIDATOR",
        last_command_completed_at: "2026-04-27T10:00:00Z",
      },
      {
        session_key: currentSessionId,
        session_id: currentSessionId,
        wp_id: fixture.wpId,
        role: "VALIDATOR",
        last_event_at: "2026-04-27T11:00:00Z",
      },
    ],
  }, null, 2)}\n`);

  const result = runSelfPrime(fixture, "VALIDATOR");

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /<predecessor-summary/);
  assert.match(result.stdout, /PREDECESSOR_SESSION_ID: VALIDATOR:WP-TEST-SELF-PRIME-v1:previous/);
  assert.match(result.stdout, /prior_manual_validation_check/);
});

test("role-self-prime PreCompact writes a fresh prompt prefix to the summary file", () => {
  const fixture = createFixture();
  const summaryPath = path.join(fixture.root, "compact-summary.md");
  const sessionId = `CODER:${fixture.wpId}`;
  appendJsonl(fixtureSessionEventsFile(fixture, sessionId), {
    schema_id: "hsk.session_event@1",
    schema_version: "session_event_v1",
    timestamp: "2026-04-27T10:00:00Z",
    wp_id: fixture.wpId,
    role: "CODER",
    session_id: sessionId,
    event_type: "steer_received",
    source_role: "ORCHESTRATOR",
    summary: "precompact state should be preserved",
  });
  writeText(fixture.registryPath, `${JSON.stringify({
    sessions: [
      {
        session_key: sessionId,
        session_id: sessionId,
        wp_id: fixture.wpId,
        role: "CODER",
        last_event_at: "2026-04-27T10:00:00Z",
      },
    ],
  }, null, 2)}\n`);
  const result = runSelfPrime(fixture, "CODER", [
    "--event", "PreCompact",
    "--write-summary", summaryPath,
    "--json",
  ]);

  assert.equal(result.status, 0, result.stderr || result.stdout);
  const payload = JSON.parse(result.stdout);
  assert.equal(payload.event, "PreCompact");
  assert.match(payload.prompt, /ROLE_SELF_PRIME \[RGF-246\]/);
  const summary = fs.readFileSync(summaryPath, "utf8");
  assert.match(summary, /ROLE_SELF_PRIME_PRECOMPACT \[RGF-246\]/);
  assert.match(summary, /<predecessor-summary/);
  assert.match(summary, /precompact state should be preserved/);
  assert.match(summary, /ROLE_SELF_PRIME_EFFECTIVE_PROMPT:/);
});
