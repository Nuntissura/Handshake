import fs from "node:fs";
import path from "node:path";

export const PRECREATED_MICROTASK_ADOPTION_MODE = "ADOPT_PRECREATED_SUITE";
export const LEGACY_MICROTASK_GENERATION_MODE = "LEGACY_GENERATE";

const MICRO_TASK_SCHEMA_ID = "hsk.microtask_contract@1";
const MICRO_TASK_SCHEMA_VERSION = "microtask_contract_v1";
const MT_ID_RE = /^MT-\d{3,}$/;
const MT_JSON_CANDIDATE_RE = /^MT-.*\.json$/i;

function fail(message) {
  throw new Error(`Pre-created microtask adoption blocked: ${message}`);
}

function readJson(filePath, label) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    fail(`${label} is not valid JSON (${error?.message || error})`);
  }
}

function requireStringArray(value, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  const normalized = value.map((entry) => String(entry || "").trim());
  if (normalized.some((entry) => !entry)) fail(`${label} contains an empty value`);
  if (new Set(normalized).size !== normalized.length) fail(`${label} contains duplicate values`);
  return normalized;
}

function sameArray(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function expectedIdsFromRange(range) {
  const match = String(range || "").trim().match(/^(MT-(\d{3,}))\.\.(MT-(\d{3,}))$/);
  if (!match) fail(`index range must use MT-NNN..MT-NNN syntax (found ${range || "<missing>"})`);
  const first = Number.parseInt(match[2], 10);
  const last = Number.parseInt(match[4], 10);
  if (first > last) fail(`index range start ${match[1]} is after ${match[3]}`);
  return Array.from({ length: last - first + 1 }, (_, index) => `MT-${String(first + index).padStart(3, "0")}`);
}

function validateAcyclicGraph(ids, dependenciesById) {
  const visiting = new Set();
  const visited = new Set();
  const stack = [];

  const visit = (id) => {
    if (visited.has(id)) return;
    if (visiting.has(id)) {
      const cycleStart = stack.indexOf(id);
      fail(`dependency cycle detected: ${[...stack.slice(cycleStart), id].join(" -> ")}`);
    }
    visiting.add(id);
    stack.push(id);
    for (const dependency of dependenciesById.get(id) || []) visit(dependency);
    stack.pop();
    visiting.delete(id);
    visited.add(id);
  };

  for (const id of ids) visit(id);
}

/**
 * Inspect and validate a pre-created MT contract suite without writing to it.
 *
 * An MT JSON file is an adoption signal. Once present, `_MT_INDEX.json` and the
 * complete indexed suite are mandatory. Invalid or partial suites fail closed;
 * only a directory with no MT JSON candidates retains legacy generation.
 */
export function inspectPrecreatedMicrotaskSuite({ wpDir, wpId }) {
  if (!wpDir || !wpId) fail("wpDir and wpId are required");
  if (!fs.existsSync(wpDir)) {
    return {
      mode: LEGACY_MICROTASK_GENERATION_MODE,
      declaredIds: [],
      activeId: null,
      nextId: null,
      contractsById: new Map(),
    };
  }

  const candidateFiles = fs.readdirSync(wpDir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && MT_JSON_CANDIDATE_RE.test(entry.name))
    .map((entry) => entry.name)
    .sort();

  if (candidateFiles.length === 0) {
    return {
      mode: LEGACY_MICROTASK_GENERATION_MODE,
      declaredIds: [],
      activeId: null,
      nextId: null,
      contractsById: new Map(),
    };
  }

  for (const fileName of candidateFiles) {
    const id = fileName.replace(/\.json$/i, "");
    if (!MT_ID_RE.test(id) || fileName !== `${id}.json`) {
      fail(`malformed MT contract filename ${fileName}; expected MT-NNN.json`);
    }
  }

  const indexPath = path.join(wpDir, "_MT_INDEX.json");
  if (!fs.existsSync(indexPath)) fail("_MT_INDEX.json is required when MT JSON contracts already exist");
  const index = readJson(indexPath, "_MT_INDEX.json");
  if (!index || typeof index !== "object" || Array.isArray(index)) fail("_MT_INDEX.json must contain an object");
  if (String(index.wp_id || "").trim() !== wpId) {
    fail(`index wp_id mismatch (expected ${wpId}, found ${index.wp_id || "<missing>"})`);
  }
  if (!Array.isArray(index.microtasks) || index.microtasks.length === 0) {
    fail("index microtasks must be a non-empty array");
  }

  const declaredIds = index.microtasks.map((row, indexNumber) => {
    const id = String(row?.mt_id || "").trim();
    if (!MT_ID_RE.test(id)) fail(`index row ${indexNumber + 1} has invalid mt_id ${id || "<missing>"}`);
    return id;
  });
  if (new Set(declaredIds).size !== declaredIds.length) fail("index contains duplicate mt_id values");
  if (index.total !== undefined && index.total !== declaredIds.length) {
    fail(`index total mismatch (expected ${declaredIds.length}, found ${index.total})`);
  }
  if (index.range !== undefined) {
    const rangeIds = expectedIdsFromRange(index.range);
    if (!sameArray(declaredIds, rangeIds)) {
      fail(`index ids do not exactly cover declared range ${index.range}`);
    }
  }

  const diskIds = candidateFiles.map((fileName) => fileName.slice(0, -5));
  const declaredSet = new Set(declaredIds);
  const diskSet = new Set(diskIds);
  const missingFiles = declaredIds.filter((id) => !diskSet.has(id));
  const extraFiles = diskIds.filter((id) => !declaredSet.has(id));
  if (missingFiles.length > 0 || extraFiles.length > 0) {
    fail([
      missingFiles.length > 0 ? `missing indexed contracts: ${missingFiles.join(", ")}` : "",
      extraFiles.length > 0 ? `unindexed contracts: ${extraFiles.join(", ")}` : "",
    ].filter(Boolean).join("; "));
  }

  const contractsById = new Map();
  const dependenciesById = new Map();
  const activeIds = [];

  for (let indexNumber = 0; indexNumber < index.microtasks.length; indexNumber += 1) {
    const indexRow = index.microtasks[indexNumber];
    const id = declaredIds[indexNumber];
    const contractPath = path.join(wpDir, `${id}.json`);
    const contract = readJson(contractPath, `${id}.json`);
    if (!contract || typeof contract !== "object" || Array.isArray(contract)) fail(`${id}.json must contain an object`);
    if (contract.schema_id !== MICRO_TASK_SCHEMA_ID) {
      fail(`${id}: schema_id mismatch (expected ${MICRO_TASK_SCHEMA_ID}, found ${contract.schema_id || "<missing>"})`);
    }
    if (contract.schema_version !== MICRO_TASK_SCHEMA_VERSION) {
      fail(`${id}: schema_version mismatch (expected ${MICRO_TASK_SCHEMA_VERSION}, found ${contract.schema_version || "<missing>"})`);
    }
    if (contract.wp_id !== wpId || contract.mt_id !== id) {
      fail(`${id}: identity mismatch (expected ${wpId}/${id}, found ${contract.wp_id || "<missing>"}/${contract.mt_id || "<missing>"})`);
    }
    if (!contract.lifecycle || typeof contract.lifecycle !== "object" || Array.isArray(contract.lifecycle)) {
      fail(`${id}: lifecycle object is required`);
    }
    if (typeof contract.lifecycle.active !== "boolean") fail(`${id}: lifecycle.active must be boolean`);
    if (contract.lifecycle.active) activeIds.push(id);

    const contractDependencies = requireStringArray(contract.lifecycle.depends_on, `${id}.lifecycle.depends_on`);
    for (const dependency of contractDependencies) {
      if (!MT_ID_RE.test(dependency)) fail(`${id}: invalid dependency id ${dependency}`);
      if (dependency === id) fail(`${id}: self dependency is not allowed`);
      if (!declaredSet.has(dependency)) fail(`${id}: dependency ${dependency} is missing from the indexed suite`);
    }
    const indexDependencies = requireStringArray(indexRow?.depends_on, `index ${id}.depends_on`);
    if (!sameArray(indexDependencies, contractDependencies)) {
      fail(`${id}: index depends_on does not match lifecycle.depends_on`);
    }
    if (indexRow.status !== undefined && indexRow.status !== contract.lifecycle.status) {
      fail(`${id}: index status does not match lifecycle.status`);
    }
    if (indexRow.summary !== undefined && indexRow.summary !== contract.scope?.summary) {
      fail(`${id}: index summary does not match scope.summary`);
    }

    contractsById.set(id, contract);
    dependenciesById.set(id, contractDependencies);
  }

  if (activeIds.length > 1) fail(`multiple contracts are active: ${activeIds.join(", ")}`);
  validateAcyclicGraph(declaredIds, dependenciesById);

  const activeId = activeIds[0] || null;
  const firstRootId = declaredIds.find((id) => (dependenciesById.get(id) || []).length === 0) || null;
  const nextId = activeId || firstRootId;
  if (!nextId) fail("suite has no dependency-free MT available for next_id");

  return {
    mode: PRECREATED_MICROTASK_ADOPTION_MODE,
    indexPath,
    declaredIds,
    activeId,
    nextId,
    contractsById,
  };
}
