#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const RECEIPT_KIND = "HBR_VIOLATION";
export const SCHEMA_VERSION = 1;
export const ROLES = Object.freeze([
  "ORCHESTRATOR",
  "KERNEL_BUILDER",
  "CODER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "VALIDATOR",
  "CLASSIC_ORCHESTRATOR",
]);
export const EVALUATION_POINTS = Object.freeze(["build", "handoff"]);
export const VIOLATION_CLASSES = Object.freeze([
  "MISSING_EVIDENCE",
  "EVIDENCE_KIND_MISMATCH",
  "EVIDENCE_PROOF_FAILED",
  "APPLICABILITY_MISCONFIG",
  "DOWNGRADE_ATTEMPT",
  "MATRIX_SCHEMA_VIOLATION",
]);
export const CANONICAL_KEYS = Object.freeze([
  "emitted_at_utc",
  "evaluation_point",
  "evidence_pointer",
  "hbr_id",
  "mt_id",
  "notes",
  "receipt_kind",
  "receipt_uuid",
  "role",
  "schema_version",
  "source_session",
  "violation_class",
  "wp_id",
]);

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCRIPT_DIR = path.dirname(SCRIPT_PATH);
const SCHEMA_PATH = path.resolve(SCRIPT_DIR, "../schemas/hbr-violation.schema.json");
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

export function buildHbrViolation(input = {}) {
  return {
    receipt_kind: input.receipt_kind ?? RECEIPT_KIND,
    schema_version: input.schema_version ?? SCHEMA_VERSION,
    receipt_uuid: input.receipt_uuid ?? uuidV7(),
    hbr_id: input.hbr_id ?? "",
    wp_id: input.wp_id ?? "",
    mt_id: normalizeNullable(input.mt_id),
    role: input.role ?? "",
    evaluation_point: input.evaluation_point ?? "",
    evidence_pointer: normalizeNullable(input.evidence_pointer),
    violation_class: input.violation_class ?? "",
    emitted_at_utc: input.emitted_at_utc ?? isoSeconds(),
    source_session: normalizeNullable(input.source_session),
    notes: normalizeNullable(input.notes),
  };
}

function typeMatches(value, allowedType) {
  if (Array.isArray(allowedType)) {
    return allowedType.some((entry) => typeMatches(value, entry));
  }
  if (allowedType === "null") return value === null;
  if (allowedType === "array") return Array.isArray(value);
  if (allowedType === "object") return Boolean(value && typeof value === "object" && !Array.isArray(value));
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

export function validateHbrViolation(violation, schema = readSchema()) {
  const errors = [];
  if (!violation || typeof violation !== "object" || Array.isArray(violation)) {
    return ["HBR_VIOLATION must be an object"];
  }

  for (const key of schema.required || []) {
    if (!(key in violation)) errors.push(`${key} is required`);
  }
  if (schema.additionalProperties === false) {
    for (const key of Object.keys(violation)) {
      if (!schema.properties[key]) errors.push(`${key} is not allowed`);
    }
  }
  for (const [name, propertySchema] of Object.entries(schema.properties || {})) {
    if (name in violation) validateProperty(name, violation[name], propertySchema, errors);
  }
  if (violation.receipt_uuid && !UUID_V7_RE.test(String(violation.receipt_uuid))) {
    errors.push("receipt_uuid must be UUID v7");
  }
  return errors;
}

export function assertValidHbrViolation(violation) {
  const errors = validateHbrViolation(violation);
  if (errors.length > 0) {
    throw new Error(`HBR_VIOLATION schema validation failed: ${errors.join("; ")}`);
  }
}

export function canonicalObject(violation) {
  assertValidHbrViolation(violation);
  const sorted = {};
  for (const key of CANONICAL_KEYS) {
    sorted[key] = violation[key];
  }
  return sorted;
}

export function canonicalJsonLine(violation) {
  return `${JSON.stringify(canonicalObject(violation))}\n`;
}

export function emitHbrViolation(violation, { outPath = "", sink = null } = {}) {
  const line = canonicalJsonLine(violation);
  if (sink && typeof sink.write === "function") {
    sink.write(line);
  }
  if (outPath) {
    fs.mkdirSync(path.dirname(path.resolve(outPath)), { recursive: true });
    fs.appendFileSync(path.resolve(outPath), line, "utf8");
  }
  return line;
}

function fixtureViolation(overrides = {}) {
  return buildHbrViolation({
    receipt_uuid: "018f6d3a-1f00-7a2b-8c3d-123456789abc",
    hbr_id: "HBR-INT-001",
    wp_id: "WP-KERNEL-004-TEST",
    mt_id: "MT-006",
    role: "KERNEL_BUILDER",
    evaluation_point: "build",
    evidence_pointer: "test://hbr_violation_wire_contract",
    violation_class: "MISSING_EVIDENCE",
    emitted_at_utc: "2026-05-18T00:00:00Z",
    source_session: "KERNEL_BUILDER-20260518-012310",
    notes: "wire contract fixture",
    ...overrides,
  });
}

function selfTest() {
  const expected = "{\"emitted_at_utc\":\"2026-05-18T00:00:00Z\",\"evaluation_point\":\"build\",\"evidence_pointer\":\"test://hbr_violation_wire_contract\",\"hbr_id\":\"HBR-INT-001\",\"mt_id\":\"MT-006\",\"notes\":\"wire contract fixture\",\"receipt_kind\":\"HBR_VIOLATION\",\"receipt_uuid\":\"018f6d3a-1f00-7a2b-8c3d-123456789abc\",\"role\":\"KERNEL_BUILDER\",\"schema_version\":1,\"source_session\":\"KERNEL_BUILDER-20260518-012310\",\"violation_class\":\"MISSING_EVIDENCE\",\"wp_id\":\"WP-KERNEL-004-TEST\"}\n";
  const canonical = canonicalJsonLine(fixtureViolation());
  if (canonical !== expected) {
    throw new Error("canonical fixture drift");
  }
  for (const violationClass of VIOLATION_CLASSES) {
    assertValidHbrViolation(fixtureViolation({ violation_class: violationClass }));
  }
  const minted = buildHbrViolation({
    hbr_id: "HBR-INT-008",
    wp_id: "WP-KERNEL-004-TEST",
    role: "KERNEL_BUILDER",
    evaluation_point: "build",
    violation_class: "MISSING_EVIDENCE",
  });
  assertValidHbrViolation(minted);
  console.log("[HBR_VIOLATION_EMIT] self-test ok");
}

function parseArgs(argv) {
  return {
    selfTest: argv.includes("--self-test"),
    normalizeStdin: argv.includes("--normalize-stdin"),
  };
}

export function runCli(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  try {
    if (args.selfTest) {
      selfTest();
      return 0;
    }
    if (args.normalizeStdin) {
      const input = fs.readFileSync(0, "utf8").trim();
      process.stdout.write(canonicalJsonLine(JSON.parse(input)));
      return 0;
    }
    console.error("Usage: node hbr-violation-emit.mjs --self-test | --normalize-stdin");
    return 3;
  } catch (error) {
    console.error(`[HBR_VIOLATION_EMIT] ${error instanceof Error ? error.message : String(error)}`);
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
