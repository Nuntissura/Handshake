#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  repoPathAbs,
  resolveWorkPacketPath,
  workPacketPath,
} from "../lib/runtime-paths.mjs";
import { buildEphemeralContextBlock } from "./ephemeral-injection-lib.mjs";
import {
  buildInlineStartupPrompt,
  resolveRoleConfig,
  resolveRoleLaunchSelection,
} from "./session-control-lib.mjs";
import { buildActiveLaneBrief, formatActiveLaneBrief } from "./active-lane-brief-lib.mjs";

const SELF_PRIME_SCHEMA_ID = "hsk.role_self_prime@1";
const SELF_PRIME_SCHEMA_VERSION = "role_self_prime_v1";

function normalizeRole(value = "") {
  return String(value || "").trim().toUpperCase();
}

function normalizeText(value = "", fallback = "<none>") {
  const text = String(value || "").trim();
  return text || fallback;
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function readTextIfExists(filePath = "") {
  if (!filePath) return "";
  const absPath = path.isAbsolute(filePath) ? filePath : repoPathAbs(filePath);
  if (!fs.existsSync(absPath)) return "";
  return fs.readFileSync(absPath, "utf8");
}

function readJsonIfExists(filePath = "") {
  const text = readTextIfExists(filePath);
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function candidateTerminalRecordPaths(wpId) {
  return [
    path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "WP_COMMUNICATIONS", wpId, "TERMINAL_CLOSEOUT_RECORD.json"),
    path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "WP_COMMUNICATIONS", wpId, "TERMINAL_RECORD.json"),
    repoPathAbs(`.GOV/task_packets/${wpId}/TERMINAL_CLOSEOUT_RECORD.json`),
  ];
}

function summarizeTerminalCloseout(wpId) {
  for (const candidate of candidateTerminalRecordPaths(wpId)) {
    const record = readJsonIfExists(candidate);
    if (!record) continue;
    const verdict = record.verdict || record.product_verdict || record.status || record.outcome || "<unknown>";
    const updated = record.updated_at || record.updated_at_utc || record.published_at || record.timestamp_utc || "<unknown>";
    return {
      status: "PRESENT",
      path: candidate,
      summary: `verdict=${normalizeText(verdict)} updated_at=${normalizeText(updated)}`,
    };
  }
  return {
    status: "ABSENT",
    path: "<none>",
    summary: "RGF-233 terminal closeout record not present; fallback to packet/runtime projections.",
  };
}

function summarizePacketProjection(wpId) {
  const packetInfo = resolveWorkPacketPath(wpId);
  const packetPath = packetInfo?.packetPath || workPacketPath(wpId);
  const packetText = readTextIfExists(packetPath);
  if (!packetText) {
    return {
      status: "ABSENT",
      path: packetPath,
      summary: "packet not found",
    };
  }
  const status = parseSingleField(packetText, "Status");
  const lane = parseSingleField(packetText, "WORKFLOW_LANE");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  return {
    status: "PRESENT",
    path: packetPath,
    summary: [
      `status=${normalizeText(status)}`,
      `lane=${normalizeText(lane)}`,
      `runtime=${normalizeText(runtimeStatusFile)}`,
      `receipts=${normalizeText(receiptsFile)}`,
    ].join(" | "),
  };
}

function summarizeRuntimeStatus(packetSummary) {
  const runtimePath = (packetSummary.summary.match(/\bruntime=([^|]+)/) || [])[1]?.trim();
  const runtime = runtimePath && runtimePath !== "<none>" ? readJsonIfExists(runtimePath) : null;
  if (!runtime) {
    return {
      status: "ABSENT",
      path: normalizeText(runtimePath),
      summary: "runtime projection not found or unreadable",
    };
  }
  return {
    status: "PRESENT",
    path: runtimePath,
    summary: [
      `phase=${normalizeText(runtime.current_phase || runtime.phase)}`,
      `milestone=${normalizeText(runtime.current_milestone || runtime.milestone)}`,
      `next=${normalizeText(runtime.next_expected_actor)}:${normalizeText(runtime.next_expected_session)}`,
      `waiting_on=${normalizeText(runtime.waiting_on)}:${normalizeText(runtime.waiting_on_session)}`,
    ].join(" | "),
  };
}

function summarizeMicrotaskBoard(wpId, mtId = "") {
  const packetDir = repoPathAbs(`.GOV/task_packets/${wpId}`);
  if (!fs.existsSync(packetDir)) {
    return {
      status: "ABSENT",
      path: `.GOV/task_packets/${wpId}`,
      summary: "microtask packet directory not found",
    };
  }
  const mtFiles = fs.readdirSync(packetDir)
    .filter((name) => /^MT-\d+.*\.md$/i.test(name))
    .sort();
  const requestedMt = String(mtId || "").trim();
  const requestedFile = requestedMt
    ? mtFiles.find((name) => name.toUpperCase().startsWith(requestedMt.toUpperCase()))
    : "";
  const requestedText = requestedFile ? readTextIfExists(path.join(packetDir, requestedFile)) : "";
  const title = requestedText.split(/\r?\n/, 1)[0]?.replace(/^#+\s*/, "").trim() || "";
  return {
    status: mtFiles.length > 0 ? "PRESENT" : "EMPTY",
    path: `.GOV/task_packets/${wpId}`,
    summary: [
      `declared_count=${mtFiles.length}`,
      `requested=${requestedMt || "<none>"}`,
      `requested_found=${requestedFile ? "YES" : (requestedMt ? "NO" : "N/A")}`,
      title ? `requested_title=${title}` : "",
    ].filter(Boolean).join(" | "),
  };
}

function buildActiveLaneSection({ role, wpId }) {
  if (!["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(role)) {
    return {
      status: "SKIPPED",
      text: `ACTIVE_LANE_BRIEF: skipped for ${role}; role uses its own startup/next surface.`,
    };
  }
  try {
    const brief = buildActiveLaneBrief({ role, wpId });
    return {
      status: "PRESENT",
      text: formatActiveLaneBrief(brief).trimEnd(),
    };
  } catch (error) {
    return {
      status: "ERROR",
      text: `ACTIVE_LANE_BRIEF: unavailable (${error?.message || error})`,
    };
  }
}

function buildCanonicalContext({
  role,
  wpId,
  mtId = "",
  sessionId = "",
  event = "SessionStart",
} = {}) {
  const terminal = summarizeTerminalCloseout(wpId);
  const packet = summarizePacketProjection(wpId);
  const runtime = summarizeRuntimeStatus(packet);
  const microtasks = summarizeMicrotaskBoard(wpId, mtId);
  const activeLane = buildActiveLaneSection({ role, wpId });
  const lines = [
    `ROLE_SELF_PRIME [RGF-246]`,
    `- SCHEMA: ${SELF_PRIME_SCHEMA_ID} (${SELF_PRIME_SCHEMA_VERSION})`,
    `- EVENT: ${normalizeText(event, "SessionStart")}`,
    `- ROLE: ${role}`,
    `- WP_ID: ${wpId}`,
    `- MT_ID: ${normalizeText(mtId)}`,
    `- SESSION_ID: ${normalizeText(sessionId)}`,
    `- SOURCE_PRIORITY: terminal_closeout_record -> packet_projection -> mt_board -> runtime_status -> repomem/governance_memory`,
    `- TERMINAL_CLOSEOUT_RECORD: ${terminal.status} | path=${terminal.path} | ${terminal.summary}`,
    `- PACKET_PROJECTION: ${packet.status} | path=${packet.path} | ${packet.summary}`,
    `- RUNTIME_STATUS: ${runtime.status} | path=${runtime.path} | ${runtime.summary}`,
    `- MT_BOARD: ${microtasks.status} | path=${microtasks.path} | ${microtasks.summary}`,
    `- MEMORY_POLICY: bounded repomem/governance-memory injection is included by the inline startup builder below; live packet/runtime truth wins over memory hints.`,
    "",
    activeLane.text,
  ];

  return buildEphemeralContextBlock({
    source: "role-self-prime",
    trust: "required",
    body: lines.join("\n"),
  });
}

export async function rolePrime({
  role = "",
  wpId = "",
  mtId = "",
  sessionId = "",
  event = "SessionStart",
  modelSelector = "PRIMARY",
} = {}) {
  const normalizedRole = normalizeRole(role);
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedRole) throw new Error("rolePrime requires role");
  if (!normalizedWpId || !normalizedWpId.startsWith("WP-")) throw new Error("rolePrime requires WP_ID");

  const roleConfig = resolveRoleConfig(normalizedRole, normalizedWpId);
  if (!roleConfig) throw new Error(`Unknown role: ${normalizedRole}`);

  const selection = resolveRoleLaunchSelection({
    role: normalizedRole,
    wpId: normalizedWpId,
    modelSelector,
  });
  const selectedProfile = selection.selectedProfile || null;
  const selectedModel = selectedProfile?.launch_model || modelSelector || "PRIMARY";
  const contextBlock = buildCanonicalContext({
    role: normalizedRole,
    wpId: normalizedWpId,
    mtId,
    sessionId,
    event,
  });
  const inlinePrompt = buildInlineStartupPrompt({
    role: normalizedRole,
    wpId: normalizedWpId,
    roleConfig,
    selectedModel,
    selectedProfileId: selection.selectedProfileId || "",
    selectedProfile,
  });

  return [
    contextBlock,
    "",
    "ROLE_SELF_PRIME_EFFECTIVE_PROMPT:",
    inlinePrompt,
  ].join("\n");
}

function parseArgs(argv = process.argv.slice(2)) {
  const args = {
    role: process.env.HANDSHAKE_ROLE || "",
    wpId: process.env.WP_ID || "",
    mtId: process.env.MT_ID || "",
    sessionId: process.env.SESSION_ID || "",
    event: process.env.HANDSHAKE_HOOK_EVENT || "SessionStart",
    modelSelector: process.env.HANDSHAKE_MODEL_SELECTOR || "PRIMARY",
    writeSummary: process.env.COMPACTION_SUMMARY_FILE || "",
    json: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = String(argv[index] || "").trim();
    const next = () => {
      const candidate = String(argv[index + 1] || "").trim();
      if (!candidate || candidate.startsWith("--")) return "";
      index += 1;
      return candidate;
    };
    if (arg === "--role") args.role = next();
    else if (arg === "--wp-id") args.wpId = next();
    else if (arg === "--mt-id") args.mtId = next();
    else if (arg === "--session-id") args.sessionId = next();
    else if (arg === "--event") args.event = next();
    else if (arg === "--model-selector") args.modelSelector = next();
    else if (arg === "--write-summary") args.writeSummary = next();
    else if (arg === "--json") args.json = true;
    else if (arg === "--help" || arg === "-h") args.help = true;
  }
  return args;
}

function resolveWritePath(filePath = "") {
  const text = String(filePath || "").trim();
  if (!text) return "";
  return path.isAbsolute(text) ? text : repoPathAbs(text);
}

export async function runRoleSelfPrimeCli(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  if (args.help || !args.role || !args.wpId) {
    console.error("Usage: node .GOV/roles_shared/scripts/session/role-self-prime.mjs --role ROLE --wp-id WP-{ID} [--mt-id MT-NNN] [--session-id ROLE:WP] [--event SessionStart|PreCompact] [--write-summary FILE] [--json]");
    process.exit(args.help ? 0 : 1);
  }
  const prompt = await rolePrime(args);
  const writePath = resolveWritePath(args.writeSummary);
  if (writePath) {
    fs.mkdirSync(path.dirname(writePath), { recursive: true });
    fs.appendFileSync(
      writePath,
      `\n\nROLE_SELF_PRIME_PRECOMPACT [RGF-246]\n${prompt}\n`,
      "utf8",
    );
  }
  if (args.json) {
    process.stdout.write(`${JSON.stringify({
      schema_id: SELF_PRIME_SCHEMA_ID,
      schema_version: SELF_PRIME_SCHEMA_VERSION,
      role: normalizeRole(args.role),
      wp_id: args.wpId,
      mt_id: args.mtId || "",
      session_id: args.sessionId || "",
      event: args.event || "SessionStart",
      write_summary: writePath || "",
      prompt,
    }, null, 2)}\n`);
  } else {
    process.stdout.write(`${prompt}\n`);
  }
}

function sameCliPath(left, right) {
  const leftPath = path.resolve(left);
  const rightPath = path.resolve(right);
  if (leftPath === rightPath) return true;
  try {
    return fs.realpathSync.native(leftPath) === fs.realpathSync.native(rightPath);
  } catch {
    return false;
  }
}

const isMain = process.argv[1] && sameCliPath(process.argv[1], fileURLToPath(import.meta.url));
if (isMain) {
  runRoleSelfPrimeCli().catch((error) => {
    console.error(`[ROLE_SELF_PRIME] ${error?.message || error}`);
    process.exit(1);
  });
}
