import crypto from "node:crypto";

export const WORK_PACKET_CONTRACT_SCHEMA_ID = "hsk.work_packet_contract@1";
export const REFINEMENT_CONTRACT_SCHEMA_ID = "hsk.refinement_contract@1";
export const MICRO_TASK_CONTRACT_SCHEMA_ID = "hsk.microtask_contract@1";
export const DETERMINISTIC_CONTRACT_RED_TEAM_PROFILE = "DETERMINISTIC_CONTRACT_MIGRATION_V1";
export const GENERATED_PROJECTION_HEADER_RE = /^\s*<!--\s*HANDSHAKE_GENERATED_PROJECTION\s+([^]*?)-->\s*\r?\n?/;

function stableCopy(value) {
  if (Array.isArray(value)) return value.map((entry) => stableCopy(entry));
  if (!value || typeof value !== "object") return value;
  const out = {};
  for (const key of Object.keys(value).sort()) {
    out[key] = stableCopy(value[key]);
  }
  return out;
}

export function stableStringify(value) {
  return `${JSON.stringify(stableCopy(value), null, 2)}\n`;
}

export function sha256Hex(value = "") {
  return crypto.createHash("sha256").update(String(value || ""), "utf8").digest("hex");
}

export function sha256Short(value = "") {
  return sha256Hex(value).slice(0, 16);
}

export function normalizeContractPath(value = "") {
  return String(value || "").replace(/\\/g, "/").replace(/\/+/g, "/").trim();
}

function cloneForSourceHash(contract = {}) {
  const cloned = stableCopy(contract || {});
  if (!cloned.markdown_projection || typeof cloned.markdown_projection !== "object") {
    cloned.markdown_projection = {};
  }
  cloned.markdown_projection.source_hash = null;
  cloned.markdown_projection.projection_hash = null;
  cloned.markdown_projection.generated_at_utc = null;
  return cloned;
}

export function contractSourceHash(contract = {}) {
  return sha256Short(stableStringify(cloneForSourceHash(contract)));
}

export function stripGeneratedProjectionHeader(markdown = "") {
  return String(markdown || "").replace(GENERATED_PROJECTION_HEADER_RE, "");
}

export function projectionBodyHash(markdown = "") {
  return sha256Short(stripGeneratedProjectionHeader(markdown));
}

export function stampContractProjectionMetadata(contract = {}, {
  projectionPath = "",
  projectionBody = "",
  sourceFile = "",
  generator = "",
  generatedAtUtc = "",
} = {}) {
  const stamped = stableCopy(contract || {});
  stamped.contract_authority = stamped.contract_authority || "PRIMARY_MACHINE_READABLE";
  stamped.markdown_projection = {
    ...(stamped.markdown_projection && typeof stamped.markdown_projection === "object" ? stamped.markdown_projection : {}),
    path: normalizeContractPath(projectionPath || stamped.markdown_projection?.path || ""),
    status: "GENERATED_IN_SYNC",
    source_file: normalizeContractPath(sourceFile || stamped.markdown_projection?.source_file || ""),
    source_hash: null,
    projection_hash: projectionBodyHash(projectionBody),
    generated_at_utc: generatedAtUtc || new Date().toISOString(),
    generator: generator || stamped.markdown_projection?.generator || "unknown",
  };
  stamped.markdown_projection.source_hash = contractSourceHash(stamped);
  return stamped;
}

export function projectionHeaderForContract(contract = {}, { sourceFile = "" } = {}) {
  const projection = contract.markdown_projection || {};
  const parts = [
    "HANDSHAKE_GENERATED_PROJECTION",
    `schema_id=${String(contract.schema_id || "").trim()}`,
    `source_file=${normalizeContractPath(sourceFile || projection.source_file || "")}`,
    `source_hash=${String(projection.source_hash || "").trim()}`,
    `projection_hash=${String(projection.projection_hash || "").trim()}`,
    `generated_at_utc=${String(projection.generated_at_utc || "").trim()}`,
    `generator=${String(projection.generator || "").trim()}`,
  ];
  return `<!-- ${parts.join(" ")} -->`;
}

export function addOrReplaceGeneratedProjectionHeader(markdown = "", contract = {}, options = {}) {
  const body = stripGeneratedProjectionHeader(markdown).replace(/^\s+/, "");
  return `${projectionHeaderForContract(contract, options)}\n${body}`;
}

export function parseGeneratedProjectionHeader(markdown = "") {
  const match = String(markdown || "").match(GENERATED_PROJECTION_HEADER_RE);
  if (!match) return null;
  const values = {};
  for (const token of String(match[1] || "").trim().split(/\s+/)) {
    const idx = token.indexOf("=");
    if (idx <= 0) continue;
    values[token.slice(0, idx)] = token.slice(idx + 1);
  }
  return values;
}

function requiredMetadataError(field, location) {
  return `${location}: markdown_projection.${field} is required for enforced generated projection checks`;
}

function isIsoTimestamp(value = "") {
  const text = String(value || "").trim();
  return Boolean(text && !Number.isNaN(Date.parse(text)) && /\d{4}-\d{2}-\d{2}T/.test(text));
}

export function validateContractProjectionPair({ contract = {}, projectionText = "", contractPath = "", projectionPath = "" } = {}) {
  const errors = [];
  if (!contract || typeof contract !== "object" || Array.isArray(contract)) {
    return ["contract must be an object"];
  }
  const projection = contract.markdown_projection || {};
  const normalizedProjectionPath = normalizeContractPath(projectionPath || projection.path || "");
  const normalizedProjectionPathFromContract = normalizeContractPath(projection.path || "");
  const normalizedSourceFile = normalizeContractPath(projection.source_file || "");
  const status = String(projection.status || "").trim().toUpperCase();
  if (status !== "GENERATED_IN_SYNC") {
    errors.push(`${contractPath}: markdown_projection.status must be GENERATED_IN_SYNC for enforced projection checks`);
  }
  for (const field of ["path", "source_file", "source_hash", "projection_hash", "generated_at_utc", "generator"]) {
    if (!String(projection[field] || "").trim()) {
      errors.push(requiredMetadataError(field, contractPath || "<contract>"));
    }
  }
  if (normalizedProjectionPath && normalizedProjectionPathFromContract && normalizedProjectionPathFromContract !== normalizedProjectionPath) {
    errors.push(`${contractPath}: markdown_projection.path drift (expected ${normalizedProjectionPath}, found ${normalizedProjectionPathFromContract})`);
  }
  if (!isIsoTimestamp(projection.generated_at_utc)) {
    errors.push(`${contractPath}: markdown_projection.generated_at_utc must be an ISO timestamp`);
  }
  const expectedSourceHash = contractSourceHash(contract);
  const expectedProjectionHash = projectionBodyHash(projectionText);
  if (String(projection.source_hash || "") !== expectedSourceHash) {
    errors.push(`${contractPath}: source_hash drift (expected ${expectedSourceHash}, found ${projection.source_hash || "<missing>"})`);
  }
  if (String(projection.projection_hash || "") !== expectedProjectionHash) {
    errors.push(`${contractPath}: projection_hash drift for ${projectionPath || projection.path || "<projection>"} (expected ${expectedProjectionHash}, found ${projection.projection_hash || "<missing>"})`);
  }
  const header = parseGeneratedProjectionHeader(projectionText);
  if (!header) {
    errors.push(`${projectionPath || projection.path || "<projection>"}: missing HANDSHAKE_GENERATED_PROJECTION header`);
  } else {
    if (header.schema_id !== String(contract.schema_id || "")) {
      errors.push(`${projectionPath || projection.path || "<projection>"}: header schema_id drift (expected ${contract.schema_id || "<missing>"}, found ${header.schema_id || "<missing>"})`);
    }
    if (header.source_file !== normalizedSourceFile) {
      errors.push(`${projectionPath || projection.path || "<projection>"}: header source_file drift (expected ${normalizedSourceFile || "<missing>"}, found ${header.source_file || "<missing>"})`);
    }
    if (header.source_hash !== expectedSourceHash) {
      errors.push(`${projectionPath || projection.path || "<projection>"}: header source_hash drift (expected ${expectedSourceHash}, found ${header.source_hash || "<missing>"})`);
    }
    if (header.projection_hash !== expectedProjectionHash) {
      errors.push(`${projectionPath || projection.path || "<projection>"}: header projection_hash drift (expected ${expectedProjectionHash}, found ${header.projection_hash || "<missing>"})`);
    }
    if (header.generated_at_utc !== String(projection.generated_at_utc || "").trim()) {
      errors.push(`${projectionPath || projection.path || "<projection>"}: header generated_at_utc drift (expected ${projection.generated_at_utc || "<missing>"}, found ${header.generated_at_utc || "<missing>"})`);
    }
    if (header.generator !== String(projection.generator || "").trim()) {
      errors.push(`${projectionPath || projection.path || "<projection>"}: header generator drift (expected ${projection.generator || "<missing>"}, found ${header.generator || "<missing>"})`);
    }
  }
  return errors;
}
