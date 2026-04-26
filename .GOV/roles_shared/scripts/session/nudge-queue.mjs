#!/usr/bin/env node

import {
  drainNudges,
  enqueueNudge,
  listQueueDepth,
} from "./nudge-queue-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("nudge-queue.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("nudge-queue.mjs", message, { role: "SHARED", details });
}

function parseJson(raw = "", label = "json") {
  try {
    return JSON.parse(raw);
  } catch (error) {
    fail(`${label} must be valid JSON: ${error.message}`);
  }
}

const [commandRaw, sessionIdRaw, ...rest] = process.argv.slice(2);
const command = String(commandRaw || "").trim().toLowerCase();
const sessionId = String(sessionIdRaw || "").trim();

if (!command || !sessionId) {
  fail("Usage: node nudge-queue.mjs <enqueue|drain|depth> <SESSION_ID> [PAYLOAD_JSON]");
}

if (command === "enqueue") {
  const payload = parseJson(rest.join(" "), "PAYLOAD_JSON");
  const result = enqueueNudge({ sessionId, payload });
  if (!result.ok) fail(`nudge enqueue failed: ${result.error}`);
  console.log(JSON.stringify({
    status: "ENQUEUED",
    session_id: sessionId,
    queue_depth: result.queueDepth,
    file_path: result.filePath,
  }));
} else if (command === "drain") {
  const result = drainNudges({ sessionId, drainerId: "nudge-queue-cli" });
  console.log(JSON.stringify({
    status: "DRAINED",
    session_id: sessionId,
    delivered: result.nudges.length,
    expired: result.expired,
    orphans_recovered: result.orphansRecovered,
    failed: result.failed.length,
    nudges: result.nudges,
  }, null, 2));
} else if (command === "depth") {
  console.log(JSON.stringify({
    status: "OK",
    session_id: sessionId,
    queue_depth: listQueueDepth(sessionId),
  }));
} else {
  fail(`Unknown nudge queue command: ${command}`);
}
