#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const RECEIPT_KIND = "HBR_VIS_GAP";
export const HBR_ID = "HBR-VIS-005";
export const SCHEMA_VERSION = 1;
export const REQUIRED_ACTION = "Remediate the missing Argus visibility/identification/steering/re-observation path in the same MT/WP when it blocks proof; otherwise record this HBR-VIS gap as a blocker with exact surface, missing Argus capability, affected proof, and recommended remediation before PASS closure.";
export const GAP_CLASSES = Object.freeze([
  "argus_cannot_see",
  "argus_cannot_identify",
  "argus_cannot_steer",
  "argus_cannot_reobserve",
  "no_cdp_handle",
  "native_child_window",
  "opaque_canvas",
  "shadow_root_inaccessible",
  "other",
]);
export const CANONICAL_KEYS = Object.freeze([
  "emitted_at_utc",
  "evidence_pointer",
  "gap_class",
  "hbr_id",
  "proposed_followup_wp",
  "receipt_kind",
  "receipt_uuid",
  "schema_version",
  "surface_name",
  "surface_path",
  "wp_id",
]);

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCRIPT_DIR = path.dirname(SCRIPT_PATH);
const SCHEMA_PATH = path.resolve(SCRIPT_DIR, "../schemas/hbr-vis-gap.schema.json");
const UUID_V7_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

function readSchema() {
  return JSON.parse(fs.readFileSync(SCHEMA_PATH, "utf8"));
}

function isoSeconds(date = new Date()) {
  return date.toISOString().replace(/\.\d{3}Z$/, "Z");
}

function uuidV7(date = new Date()) {
  const timestamp = BigInt(date.getTime());
  const bytes = crypto.randomBytes(16);
  for (let index = 5; index >= 0; index -= 1) {
    bytes[index] = Number((timestamp >> BigInt((5 - index) * 8)) & 0xffn);
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x70;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = bytes.toString("hex");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

function normalizeNullable(value) {
  return value === undefined ? null : value;
}

function isPlainObject(value) {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

export function buildHbrVisGap(input = {}) {
  return {
    receipt_kind: input.receipt_kind ?? RECEIPT_KIND,
    schema_version: input.schema_version ?? SCHEMA_VERSION,
    receipt_uuid: input.receipt_uuid ?? uuidV7(),
    hbr_id: input.hbr_id ?? HBR_ID,
    wp_id: input.wp_id ?? "",
    surface_name: input.surface_name ?? "",
    surface_path: input.surface_path ?? input.surface_name ?? "",
    gap_class: input.gap_class ?? "",
    proposed_followup_wp: normalizeNullable(input.proposed_followup_wp),
    evidence_pointer: normalizeNullable(input.evidence_pointer),
    emitted_at_utc: input.emitted_at_utc ?? isoSeconds(),
  };
}

function typeMatches(value, allowedType) {
  if (Array.isArray(allowedType)) {
    return allowedType.some((entry) => typeMatches(value, entry));
  }
  if (allowedType === "null") return value === null;
  if (allowedType === "array") return Array.isArray(value);
  if (allowedType === "object") return isPlainObject(value);
  return typeof value === allowedType;
}

function validateProperty(name, value, schema, errors) {
  if (schema.const !== undefined && value !== schema.const) {
    errors.push(`${name} must equal ${JSON.stringify(schema.const)}`);
  }
  if (schema.enum && !schema.enum.some((entry) => entry === value)) {
    errors.push(`${name} must be one of ${schema.enum.map((entry) => JSON.stringify(entry)).join(", ")}`);
  }
  if (schema.type && !typeMatches(value, schema.type)) {
    errors.push(`${name} type mismatch`);
    return;
  }
  if (schema.pattern && typeof value === "string" && !(new RegExp(schema.pattern).test(value))) {
    errors.push(`${name} does not match ${schema.pattern}`);
  }
  if (schema.minLength !== undefined && typeof value === "string" && value.length < schema.minLength) {
    errors.push(`${name} must have length >= ${schema.minLength}`);
  }
  if (schema.format === "date-time" && typeof value === "string" && Number.isNaN(Date.parse(value))) {
    errors.push(`${name} must be RFC3339 date-time`);
  }
}

export function validateHbrVisGap(gap, schema = readSchema()) {
  const errors = [];
  if (!isPlainObject(gap)) return ["HBR_VIS_GAP must be an object"];

  for (const key of schema.required || []) {
    if (!(key in gap)) errors.push(`${key} is required`);
  }
  if (schema.additionalProperties === false) {
    for (const key of Object.keys(gap)) {
      if (!schema.properties[key]) errors.push(`${key} is not allowed`);
    }
  }
  for (const [name, propertySchema] of Object.entries(schema.properties || {})) {
    if (name in gap) validateProperty(name, gap[name], propertySchema, errors);
  }
  if (gap.receipt_uuid && !UUID_V7_RE.test(String(gap.receipt_uuid))) {
    errors.push("receipt_uuid must be UUID v7");
  }
  return errors;
}

export function assertValidHbrVisGap(gap) {
  const errors = validateHbrVisGap(gap);
  if (errors.length > 0) {
    throw new Error(`HBR_VIS_GAP schema validation failed: ${errors.join("; ")}`);
  }
}

export function canonicalObject(gap) {
  assertValidHbrVisGap(gap);
  const sorted = {};
  for (const key of CANONICAL_KEYS) {
    sorted[key] = gap[key];
  }
  return sorted;
}

export function canonicalJsonLine(gap) {
  return `${JSON.stringify(canonicalObject(gap))}\n`;
}

export function blockerIdForGap(gap) {
  const digest = crypto
    .createHash("sha256")
    .update(`${gap.wp_id}|${gap.surface_path}|${gap.gap_class}`)
    .digest("hex")
    .slice(0, 12);
  return `hbr-vis-gap-${digest}`;
}

export function openBlockerForGap(gap) {
  assertValidHbrVisGap(gap);
  return {
    blocker_id: blockerIdForGap(gap),
    blocker_kind: RECEIPT_KIND,
    status: "OPEN",
    hbr_id: HBR_ID,
    wp_id: gap.wp_id,
    surface_name: gap.surface_name,
    surface_path: gap.surface_path,
    gap_class: gap.gap_class,
    receipt_uuid: gap.receipt_uuid,
    receipt_ref: `receipt://${gap.receipt_uuid}`,
    evidence_pointer: gap.evidence_pointer,
    proposed_followup_wp: gap.proposed_followup_wp,
    created_at_utc: gap.emitted_at_utc,
    required_action: REQUIRED_ACTION,
  };
}

export function appendOpenBlocker(packet, gap) {
  if (!isPlainObject(packet)) throw new Error("packet JSON must be an object");
  const blocker = openBlockerForGap(gap);
  if (packet.open_blockers === undefined) {
    packet.open_blockers = [];
  }
  if (!Array.isArray(packet.open_blockers)) {
    throw new Error("packet.open_blockers must be an array when present");
  }

  const existingIndex = packet.open_blockers.findIndex((entry) => (
    isPlainObject(entry) && entry.blocker_id === blocker.blocker_id
  ));
  if (existingIndex >= 0) {
    packet.open_blockers[existingIndex] = blocker;
  } else {
    packet.open_blockers.push(blocker);
  }
  return blocker;
}

function repoRootForPacket(packetPath) {
  const normalized = path.resolve(packetPath).replace(/\\/g, "/");
  const marker = "/.GOV/task_packets/";
  const index = normalized.indexOf(marker);
  if (index >= 0) return normalized.slice(0, index);
  return process.cwd();
}

function resolveRepoPath(repoRoot, maybeRelative) {
  if (!isNonEmptyString(maybeRelative)) return "";
  return path.isAbsolute(maybeRelative)
    ? maybeRelative
    : path.resolve(repoRoot, maybeRelative);
}

function defaultPacketPath(wpId) {
  return path.resolve(".GOV", "task_packets", wpId, "packet.json");
}

function defaultReceiptPath(repoRoot, wpId) {
  return path.resolve(repoRoot, "..", "gov_runtime", "roles_shared", "WP_COMMUNICATIONS", wpId, "RECEIPTS.jsonl");
}

function readPacket(packetPath) {
  return JSON.parse(fs.readFileSync(packetPath, "utf8"));
}

function writePacket(packetPath, packet) {
  fs.writeFileSync(packetPath, `${JSON.stringify(packet, null, 2)}\n`, "utf8");
}

function appendReceipt(receiptPath, gap) {
  fs.mkdirSync(path.dirname(receiptPath), { recursive: true });
  fs.appendFileSync(receiptPath, canonicalJsonLine(gap), "utf8");
}

function getArgValue(args, name) {
  const index = args.indexOf(name);
  if (index === -1) return undefined;
  const value = args[index + 1];
  if (!isNonEmptyString(value)) throw new Error(`${name} requires a value`);
  return value;
}

function parseArgs(argv) {
  if (argv.includes("--self-test")) return { mode: "self-test" };
  if (argv.includes("--normalize-stdin")) return { mode: "normalize-stdin" };
  return {
    mode: "emit",
    wpId: getArgValue(argv, "--wp"),
    surfaceName: getArgValue(argv, "--surface"),
    surfacePath: getArgValue(argv, "--surface-path"),
    gapClass: getArgValue(argv, "--gap-class"),
    proposedFollowupWp: getArgValue(argv, "--proposed-followup-wp"),
    evidencePointer: getArgValue(argv, "--evidence-pointer"),
    packetPath: getArgValue(argv, "--packet"),
    receiptOut: getArgValue(argv, "--receipt-out"),
    receiptUuid: getArgValue(argv, "--receipt-uuid"),
    emittedAtUtc: getArgValue(argv, "--emitted-at-utc"),
  };
}

function requireEmitArgs(args) {
  for (const [name, value] of [
    ["--wp", args.wpId],
    ["--surface", args.surfaceName],
    ["--gap-class", args.gapClass],
  ]) {
    if (!isNonEmptyString(value)) throw new Error(`${name} is required`);
  }
}

function selfTest() {
  const gap = buildHbrVisGap({
    receipt_uuid: "018f6d3a-1f00-7a2b-8c3d-123456789abc",
    wp_id: "WP-KERNEL-004-TEST",
    surface_name: "Diagnostics canvas controls",
    surface_path: "app://diagnostics/canvas-controls",
    gap_class: "opaque_canvas",
    proposed_followup_wp: "WP-KERNEL-004-VIS-GAP-FOLLOWUP-v1",
    evidence_pointer: "artifact://visual/diagnostics-canvas.png",
    emitted_at_utc: "2026-05-18T00:00:00Z",
  });
  canonicalJsonLine(gap);
  for (const gapClass of GAP_CLASSES) {
    assertValidHbrVisGap({ ...gap, gap_class: gapClass });
  }
  console.log("[HBR_VIS_GAP_EMIT] self-test ok");
}

export function runCli(argv = process.argv.slice(2)) {
  try {
    const args = parseArgs(argv);
    if (args.mode === "self-test") {
      selfTest();
      return 0;
    }
    if (args.mode === "normalize-stdin") {
      const input = fs.readFileSync(0, "utf8").trim();
      process.stdout.write(canonicalJsonLine(JSON.parse(input)));
      return 0;
    }

    requireEmitArgs(args);
    const packetPath = path.resolve(args.packetPath || defaultPacketPath(args.wpId));
    const repoRoot = repoRootForPacket(packetPath);
    const packet = readPacket(packetPath);
    const gap = buildHbrVisGap({
      receipt_uuid: args.receiptUuid,
      wp_id: args.wpId,
      surface_name: args.surfaceName,
      surface_path: args.surfacePath || args.surfaceName,
      gap_class: args.gapClass,
      proposed_followup_wp: args.proposedFollowupWp,
      evidence_pointer: args.evidencePointer,
      emitted_at_utc: args.emittedAtUtc,
    });
    assertValidHbrVisGap(gap);
    const blocker = appendOpenBlocker(packet, gap);
    const receiptPath = resolveRepoPath(
      repoRoot,
      args.receiptOut || packet.workflow?.receipts_file,
    ) || defaultReceiptPath(repoRoot, args.wpId);

    appendReceipt(receiptPath, gap);
    writePacket(packetPath, packet);
    console.log(JSON.stringify({
      receipt_kind: RECEIPT_KIND,
      receipt_uuid: gap.receipt_uuid,
      blocker_id: blocker.blocker_id,
      packet: packetPath,
      receipt_out: receiptPath,
    }));
    return 0;
  } catch (error) {
    console.error(`[HBR_VIS_GAP_EMIT] ${error instanceof Error ? error.message : String(error)}`);
    return 2;
  }
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  return fs.realpathSync.native(path.resolve(process.argv[1])) === fs.realpathSync.native(SCRIPT_PATH);
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
