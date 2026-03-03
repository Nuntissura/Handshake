#!/usr/bin/env node
/**
 * Orchestrator "resume without context" helper.
 *
 * This is intentionally read-only: it inspects Orchestrator gates + filesystem
 * state and prints the next minimal commands to run.
 */

import fs from "node:fs";
import path from "node:path";
import {
  defaultRefinementPath,
  validateRefinementFile,
} from "./validation/refinement-check.mjs";

const STATE_FILE = ".GOV/roles/orchestrator/ORCHESTRATOR_GATES.json";
const TASK_BOARD_PATH = ".GOV/roles_shared/TASK_BOARD.md";

function fail(message, details = []) {
  console.error(`[ORCHESTRATOR_NEXT] ${message}`);
  for (const line of details) console.error(`- ${line}`);
  process.exit(1);
}

function loadState() {
  if (!fs.existsSync(STATE_FILE)) return { gate_logs: [] };
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE, "utf8"));
  } catch (e) {
    fail("Failed to read ORCHESTRATOR_GATES.json", [String(e?.message || e)]);
  }
}

function lastLog(state, wpId, type) {
  const logs = Array.isArray(state.gate_logs) ? state.gate_logs : [];
  return [...logs].reverse().find((l) => l?.wpId === wpId && l?.type === type) || null;
}

function exists(p) {
  try {
    return fs.existsSync(p);
  } catch {
    return false;
  }
}

function hasStubLine(wpId) {
  if (!exists(TASK_BOARD_PATH)) return false;
  const content = fs.readFileSync(TASK_BOARD_PATH, "utf8");
  return content.includes(`- **[${wpId}]** - [STUB]`);
}

function printLifecycle({ wpId, stage, next }) {
  console.log("LIFECYCLE [CX-LIFE-001]");
  console.log(`- WP_ID: ${wpId}`);
  console.log(`- STAGE: ${stage}`);
  console.log(`- NEXT: ${next}`);
  console.log("");
}

function printOperatorAction(action) {
  console.log(`OPERATOR_ACTION: ${action || "NONE"}`);
  console.log("");
}

function printState(state) {
  console.log(`STATE: ${state}`);
  console.log("");
}

function printNextCommands(cmds) {
  console.log("NEXT_COMMANDS [CX-GATE-UX-001]");
  for (const cmd of cmds) console.log(`- ${cmd}`);
}

function main() {
  const wpId = (process.argv[2] || "").trim();
  if (!wpId || !wpId.startsWith("WP-")) {
    fail("Usage: node .GOV/scripts/orchestrator-next.mjs <WP_ID>", [
      "Example: node .GOV/scripts/orchestrator-next.mjs WP-1-ModelSession-Core-Scheduler-v1",
    ]);
  }

  const state = loadState();
  const lastRefinement = lastLog(state, wpId, "REFINEMENT");
  const lastSignature = lastLog(state, wpId, "SIGNATURE");
  const lastPrepare = lastLog(state, wpId, "PREPARE");

  const refinementPath = defaultRefinementPath(wpId);
  const packetPath = path.join(".GOV", "task_packets", `${wpId}.md`).replace(/\\/g, "/");

  const refinementExists = exists(refinementPath);
  const packetExists = exists(packetPath);

  let refinementReady = false;
  let refinementSigned = false;
  let refinementErrors = [];
  if (refinementExists) {
    const ready = validateRefinementFile(refinementPath, {
      expectedWpId: wpId,
      requireSignature: false,
    });
    refinementReady = !!ready.ok;
    if (!ready.ok) refinementErrors = ready.errors || [];

    const signed = validateRefinementFile(refinementPath, {
      expectedWpId: wpId,
      requireSignature: true,
    });
    refinementSigned = !!signed.ok;
  }

  // Phase inference (minimal and deterministic).
  if (!refinementExists) {
    printLifecycle({ wpId, stage: "REFINEMENT", next: "REFINEMENT" });
    printOperatorAction("NONE");
    printState("Refinement file does not exist yet.");
    printNextCommands([
      `just create-task-packet ${wpId}  # scaffolds .GOV/refinements/${wpId}.md and exits BLOCKED`,
      `cat ${refinementPath.replace(/\\/g, "/")}`,
      `# Present the Technical Refinement Block in-chat; wait for explicit review.`,
      `just record-refinement ${wpId}`,
    ]);
    return;
  }

  if (!refinementReady) {
    printLifecycle({ wpId, stage: "REFINEMENT", next: "REFINEMENT" });
    printOperatorAction("NONE");
    const detail = refinementErrors.length > 0 ? refinementErrors[0] : "Refinement is incomplete.";
    printState(detail);
    printNextCommands([
      `cat ${refinementPath.replace(/\\/g, "/")}`,
      `# Fix refinement fields until it is reviewable.`,
      `just record-refinement ${wpId}`,
    ]);
    return;
  }

  if (!lastRefinement) {
    printLifecycle({ wpId, stage: "REFINEMENT", next: "APPROVAL" });
    printOperatorAction("NONE");
    printState("Refinement file looks reviewable, but no refinement gate log exists yet.");
    printNextCommands([`just record-refinement ${wpId}`]);
    return;
  }

  if (!lastSignature) {
    printLifecycle({ wpId, stage: "APPROVAL", next: "SIGNATURE" });
    printOperatorAction(`Collect explicit approval + one-time signature for ${wpId}`);
    printState("Refinement recorded; signature not yet recorded.");
    printNextCommands([
      `# Ensure refinement METADATA contains: - USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT ${wpId}`,
      `just record-signature ${wpId} {usernameDDMMYYYYHHMM}`,
    ]);
    return;
  }

  if (!lastPrepare) {
    printLifecycle({ wpId, stage: "PREPARE", next: "PACKET_CREATE" });
    printOperatorAction("Choose coder assignment (Coder-A|Coder-B) for PREPARE");
    printState("Signature recorded; WP prepare record missing.");
    printNextCommands([
      `just worktree-add ${wpId}`,
      `just record-prepare ${wpId} {Coder-A|Coder-B}`,
    ]);
    return;
  }

  if (!packetExists) {
    printLifecycle({ wpId, stage: "PACKET_CREATE", next: "PRE_WORK" });
    printOperatorAction("NONE");
    printState("Prepare recorded; task packet file does not exist yet.");
    printNextCommands([
      `just create-task-packet ${wpId}`,
      `# Fill packet placeholders (AGENT_ID/REQUESTOR/SCOPE/TEST_PLAN/DONE_MEANS/BOOTSTRAP/SPEC_ANCHOR).`,
      `just pre-work ${wpId}`,
      `just task-board-set ${wpId} READY_FOR_DEV`,
    ]);
    return;
  }

  const needsStubCleanup = hasStubLine(wpId);
  printLifecycle({ wpId, stage: "DELEGATION", next: "DELEGATION" });
  printOperatorAction("NONE");
  printState(
    needsStubCleanup
      ? "Task packet exists; Task Board still lists this WP as [STUB]."
      : "Task packet exists; ready to delegate to Coder."
  );
  const cmds = [
    `cat ${packetPath}`,
    `just pre-work ${wpId}`,
  ];
  if (needsStubCleanup) cmds.push(`just task-board-set ${wpId} READY_FOR_DEV`);
  cmds.push(`# Delegate: send packet path + worktree/branch recorded in ORCHESTRATOR_GATES.json PREPARE.`);
  printNextCommands(cmds);
}

main();

