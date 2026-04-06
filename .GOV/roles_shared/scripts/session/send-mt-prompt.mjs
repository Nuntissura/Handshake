#!/usr/bin/env node
/**
 * Builds and dispatches a governed microtask prompt to the coder session.
 *
 * Auto-includes:
 * - Session keys (from registry)
 * - wp-review-request command with correct parameters
 * - "STOP and wait" instruction
 * - CARGO_TARGET_DIR hint
 *
 * Usage: node send-mt-prompt.mjs <WP_ID> <MT_ID> <description> [model]
 *
 * Example: node send-mt-prompt.mjs WP-1-FR-ModelSessionId-v1 MT-001 "Add model_session_id field to FlightRecorderEvent"
 */

import { execFileSync } from "node:child_process";
import { sessionKey } from "./session-policy.mjs";
import { REPO_ROOT } from "../lib/runtime-paths.mjs";
import path from "node:path";

const wpId = String(process.argv[2] || "").trim();
const mtId = String(process.argv[3] || "").trim();
const description = String(process.argv[4] || "").trim();
const model = String(process.argv[5] || "PRIMARY").trim();

if (!wpId || !mtId || !description) {
  console.error("Usage: node send-mt-prompt.mjs <WP_ID> <MT_ID> <description> [model]");
  console.error("Example: node send-mt-prompt.mjs WP-1-FR-ModelSessionId-v1 MT-001 \"Add model_session_id field\"");
  process.exit(1);
}

const coderKey = sessionKey("CODER", wpId);
const validatorKey = sessionKey("WP_VALIDATOR", wpId);

const prompt = [
  `MICROTASK ${mtId}: ${description}`,
  ``,
  `Your session key: ${coderKey}`,
  `Validator session key: ${validatorKey}`,
  ``,
  `Instructions:`,
  `1. Implement the microtask described above`,
  `2. Use CARGO_TARGET_DIR='../Handshake Artifacts/handshake-cargo-target' for builds`,
  `3. Commit with message: feat: ${mtId} ${description}`,
  `4. After commit, the git hook will automatically send a review request to the validator`,
  `5. STOP and wait for the validator's response before starting the next MT`,
  ``,
  `If the git hook does not fire, run manually:`,
  `just wp-review-request ${wpId} CODER ${coderKey} WP_VALIDATOR ${validatorKey} '${mtId} complete: ${description}'`,
].join("\n");

console.log(`[SEND_MT] WP: ${wpId}`);
console.log(`[SEND_MT] MT: ${mtId}`);
console.log(`[SEND_MT] Coder: ${coderKey}`);
console.log(`[SEND_MT] Validator: ${validatorKey}`);
console.log(`[SEND_MT] Dispatching...`);

try {
  const scriptPath = path.resolve(REPO_ROOT, ".GOV", "roles", "orchestrator", "scripts", "session-control-command.mjs");
  const output = execFileSync(process.execPath, [scriptPath, "SEND_PROMPT", "CODER", wpId, prompt, model], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    cwd: REPO_ROOT,
  });
  const lines = output.trim().split(/\r?\n/).filter(Boolean);
  for (const line of lines) console.log(line);
} catch (error) {
  const stderr = String(error?.stderr || "").trim();
  const stdout = String(error?.stdout || "").trim();
  console.error(`[SEND_MT] Dispatch failed: ${stderr || stdout || error.message}`);
  process.exit(1);
}
