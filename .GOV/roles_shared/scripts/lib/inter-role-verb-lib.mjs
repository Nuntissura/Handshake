import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_ABS } from "./runtime-paths.mjs";

export const INTER_ROLE_VERB_SCHEMA_VERSION = "inter_role_verb_v1";

const VERB_DEFINITIONS = Object.freeze({
  MT_HANDOFF: {
    required: ["mt_id", "range", "commit", "summary"],
    fields: {
      mt_id: { type: "string", pattern: /^MT-\d{3}$/i },
      range: { type: "string" },
      commit: { type: "string" },
      summary: { type: "string" },
    },
  },
  MT_VERDICT: {
    required: ["mt_id", "verdict", "concerns", "track"],
    fields: {
      mt_id: { type: "string", pattern: /^MT-\d{3}$/i },
      verdict: { type: "string", enum: ["PASS", "FAIL"] },
      concerns: { type: "concern_array" },
      track: { type: "string", enum: ["MECHANICAL", "JUDGMENT"] },
    },
  },
  MT_REMEDIATION_REQUIRED: {
    required: ["mt_id", "concerns", "next_action"],
    fields: {
      mt_id: { type: "string", pattern: /^MT-\d{3}$/i },
      concerns: { type: "string_array", minItems: 1 },
      next_action: { type: "string" },
    },
  },
  WP_HANDOFF: {
    required: ["wp_id", "final_range", "mts_completed", "summary"],
    fields: {
      wp_id: { type: "wp_id" },
      final_range: { type: "string" },
      mts_completed: { type: "string_array" },
      summary: { type: "string" },
    },
  },
  INTEGRATION_VERDICT: {
    required: ["wp_id", "verdict", "mechanical_track", "judgment_track", "closeout_path"],
    fields: {
      wp_id: { type: "wp_id" },
      verdict: { type: "string", enum: ["PASS", "FAIL"] },
      mechanical_track: { type: "string", enum: ["PASS", "FAIL", "PENDING", "NOT_APPLICABLE"] },
      judgment_track: { type: "string", enum: ["PASS", "FAIL", "PENDING", "NOT_APPLICABLE"] },
      closeout_path: { type: "string" },
    },
  },
  CONCERN: {
    required: ["concern_class", "severity", "evidence_path", "notes"],
    fields: {
      concern_class: { type: "string" },
      severity: { type: "string", enum: ["LOW", "MEDIUM", "HIGH", "CRITICAL"] },
      wp_id: { type: "wp_id", optional: true },
      mt_id: { type: "string", pattern: /^MT-\d{3}$/i, optional: true },
      evidence_path: { type: "string" },
      notes: { type: "string" },
    },
  },
  PHASE_TRANSITION: {
    required: ["wp_id", "from_phase", "to_phase", "provenance"],
    fields: {
      wp_id: { type: "wp_id" },
      from_phase: { type: "string" },
      to_phase: { type: "string" },
      provenance: { type: "string" },
    },
  },
  RELAUNCH_REQUEST: {
    required: ["wp_id", "target_role", "reason", "priority"],
    fields: {
      wp_id: { type: "wp_id" },
      target_role: { type: "string", enum: ["ACTIVATION_MANAGER", "CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR", "MEMORY_MANAGER"] },
      reason: { type: "string" },
      priority: { type: "string", enum: ["normal", "urgent"] },
    },
  },
});

export function interRoleVerbDefinitions() {
  return VERB_DEFINITIONS;
}

export function interRoleVerbNames() {
  return Object.keys(VERB_DEFINITIONS);
}

export function normalizeInterRoleVerb(value = "") {
  const normalized = String(value || "").trim().toUpperCase();
  return VERB_DEFINITIONS[normalized] ? normalized : "";
}

function normalizeBody(value = null) {
  if (value && typeof value === "object" && !Array.isArray(value)) return { ...value };
  return {};
}

function validateField({ verb, key, spec, value }) {
  const errors = [];
  const present = value !== undefined && value !== null && !(typeof value === "string" && value.trim() === "");
  if (!present) {
    if (!spec.optional) errors.push(`${verb}.${key} is required`);
    return errors;
  }
  if (spec.type === "string") {
    if (typeof value !== "string" || !value.trim()) errors.push(`${verb}.${key} must be a non-empty string`);
    if (spec.pattern && !spec.pattern.test(String(value || ""))) errors.push(`${verb}.${key} invalid (${value})`);
    const textValue = String(value || "").trim();
    if (spec.enum && !spec.enum.includes(textValue) && !spec.enum.includes(textValue.toUpperCase())) {
      errors.push(`${verb}.${key} must be one of ${spec.enum.join(" | ")}`);
    }
    return errors;
  }
  if (spec.type === "wp_id") {
    if (typeof value !== "string" || !/^WP-/.test(value.trim())) errors.push(`${verb}.${key} must be a WP id`);
    return errors;
  }
  if (spec.type === "string_array") {
    if (!Array.isArray(value)) {
      errors.push(`${verb}.${key} must be an array`);
      return errors;
    }
    if (Number.isInteger(spec.minItems) && value.length < spec.minItems) {
      errors.push(`${verb}.${key} must contain at least ${spec.minItems} item(s)`);
    }
    if (value.some((item) => typeof item !== "string" || !item.trim())) {
      errors.push(`${verb}.${key} must contain only non-empty strings`);
    }
    return errors;
  }
  if (spec.type === "concern_array") {
    if (!Array.isArray(value)) {
      errors.push(`${verb}.${key} must be an array`);
      return errors;
    }
    for (const [index, item] of value.entries()) {
      if (typeof item === "string") {
        if (!item.trim()) errors.push(`${verb}.${key}[${index}] must be non-empty`);
        continue;
      }
      if (!item || typeof item !== "object" || Array.isArray(item)) {
        errors.push(`${verb}.${key}[${index}] must be a string or concern object`);
        continue;
      }
      const concernKey = String(item.key || "").trim();
      const severity = String(item.severity || "").trim().toUpperCase();
      const evidencePath = String(item.evidence_path || "").trim();
      if (!concernKey) errors.push(`${verb}.${key}[${index}].key is required`);
      if (!["LOW", "MEDIUM", "HIGH", "CRITICAL"].includes(severity)) {
        errors.push(`${verb}.${key}[${index}].severity must be one of LOW | MEDIUM | HIGH | CRITICAL`);
      }
      if (!evidencePath) errors.push(`${verb}.${key}[${index}].evidence_path is required`);
    }
    return errors;
  }
  errors.push(`${verb}.${key} uses unsupported field type ${spec.type}`);
  return errors;
}

export function validateInterRoleVerbBody(verbValue = "", bodyValue = null) {
  const verb = normalizeInterRoleVerb(verbValue);
  if (!verb) {
    return {
      ok: false,
      verb: "",
      body: normalizeBody(bodyValue),
      errors: [`unknown inter-role verb: ${verbValue}`],
    };
  }
  const definition = VERB_DEFINITIONS[verb];
  const body = normalizeBody(bodyValue);
  const normalizedBody = { ...body };
  const allowed = new Set(Object.keys(definition.fields));
  const errors = [];
  for (const key of definition.required) {
    errors.push(...validateField({ verb, key, spec: definition.fields[key], value: body[key] }));
  }
  for (const [key, value] of Object.entries(body)) {
    const spec = definition.fields[key];
    if (!spec) {
      errors.push(`${verb}.${key} is not allowed`);
      continue;
    }
    if (!definition.required.includes(key)) {
      errors.push(...validateField({ verb, key, spec, value }));
    }
    if (spec?.enum) {
      const textValue = String(value || "").trim();
      if (spec.enum.includes(textValue)) normalizedBody[key] = textValue;
      else if (spec.enum.includes(textValue.toUpperCase())) normalizedBody[key] = textValue.toUpperCase();
    }
  }
  for (const key of definition.required) {
    const spec = definition.fields[key];
    const value = body[key];
    if (spec?.enum) {
      const textValue = String(value || "").trim();
      if (spec.enum.includes(textValue)) normalizedBody[key] = textValue;
      else if (spec.enum.includes(textValue.toUpperCase())) normalizedBody[key] = textValue.toUpperCase();
    }
  }
  for (const key of Object.keys(definition.fields)) {
    if (!allowed.has(key)) errors.push(`${verb}.${key} is not registered`);
  }
  return { ok: errors.length === 0, verb, body: normalizedBody, errors };
}

function compactList(value = []) {
  if (!Array.isArray(value) || value.length === 0) return "none";
  return value.map((item) => {
    if (typeof item === "string") return item;
    if (item && typeof item === "object") {
      return `${item.key || "CONCERN"}:${item.severity || "UNKNOWN"}@${item.evidence_path || "NO_EVIDENCE"}`;
    }
    return String(item || "").trim();
  }).filter(Boolean).join("; ");
}

export function renderInterRoleVerbReceipt(entry = {}) {
  const verb = normalizeInterRoleVerb(entry?.verb);
  if (!verb) return "";
  const body = normalizeBody(entry?.verb_body);
  switch (verb) {
    case "MT_HANDOFF":
      return `MT_HANDOFF ${body.mt_id}: range=${body.range}; commit=${body.commit}; ${body.summary}`;
    case "MT_VERDICT":
      return `MT_VERDICT ${body.mt_id}: ${body.verdict}/${body.track}; concerns=${compactList(body.concerns)}`;
    case "MT_REMEDIATION_REQUIRED":
      return `MT_REMEDIATION_REQUIRED ${body.mt_id}: next=${body.next_action}; concerns=${compactList(body.concerns)}`;
    case "WP_HANDOFF":
      return `WP_HANDOFF ${body.wp_id}: range=${body.final_range}; mts=${compactList(body.mts_completed)}; ${body.summary}`;
    case "INTEGRATION_VERDICT":
      return `INTEGRATION_VERDICT ${body.wp_id}: verdict=${body.verdict}; mechanical=${body.mechanical_track}; judgment=${body.judgment_track}; closeout=${body.closeout_path}`;
    case "CONCERN":
      return `CONCERN ${body.severity}/${body.concern_class}: evidence=${body.evidence_path}; ${body.notes}`;
    case "PHASE_TRANSITION":
      return `PHASE_TRANSITION ${body.wp_id}: ${body.from_phase} -> ${body.to_phase}; provenance=${body.provenance}`;
    case "RELAUNCH_REQUEST":
      return `RELAUNCH_REQUEST ${body.wp_id}: target=${body.target_role}; priority=${body.priority}; reason=${body.reason}`;
    default:
      return `${verb}: ${JSON.stringify(body)}`;
  }
}

export function schemaPathForInterRoleVerb(verbValue = "") {
  const verb = normalizeInterRoleVerb(verbValue);
  if (!verb) return "";
  return path.join(GOV_ROOT_ABS, "roles_shared", "schemas", "inter_role_verbs", `${verb}.schema.json`);
}

export function loadInterRoleVerbSchema(verbValue = "") {
  const schemaPath = schemaPathForInterRoleVerb(verbValue);
  if (!schemaPath || !fs.existsSync(schemaPath)) return null;
  return JSON.parse(fs.readFileSync(schemaPath, "utf8"));
}
