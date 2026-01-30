#!/usr/bin/env node
/**
 * Atelier role registry drift check (append-only) [WP-1]
 *
 * Enforces (per Master Spec Addendum 3.3 / 6.3.3.5.7.*):
 * - role_id set is append-only vs baseline
 * - contract_id -> schema_json hash is immutable once published
 * - role_id uniqueness
 * - contract_id format parse (ROLE:<role_id>:(X|C):<ver>)
 *
 * Baseline load strategy (locked in packet):
 * - primary: `git show main:assets/atelier_rolepack_digital_production_studio_v1.json`
 * - fallback: `git show origin/main:assets/atelier_rolepack_digital_production_studio_v1.json`
 * - if neither exists: baseline is treated as empty
 */

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const ROLEPACK_REL_PATH = "assets/atelier_rolepack_digital_production_studio_v1.json";
const ROLEPACK_PATH = path.join(repoRoot, ROLEPACK_REL_PATH);

function fail(message, details = []) {
  console.error(`[ATELIER_ROLE_REGISTRY_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function isPlainObject(value) {
  return (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    Object.prototype.toString.call(value) === "[object Object]"
  );
}

function readJsonFromFile(filePath) {
  let raw;
  try {
    raw = fs.readFileSync(filePath, "utf8");
  } catch (err) {
    fail("Cannot read rolepack JSON", [`Path: ${filePath}`, String(err?.message || err)]);
  }

  try {
    return JSON.parse(raw);
  } catch (err) {
    fail("Rolepack JSON parse failed", [`Path: ${filePath}`, String(err?.message || err)]);
  }
}

function tryGitShow(ref, relPath) {
  try {
    return execFileSync("git", ["show", `${ref}:${relPath}`], {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch {
    return null;
  }
}

function canonicalizeJson(value) {
  if (value === null) return null;
  if (typeof value !== "object") return value;
  if (Array.isArray(value)) return value.map((entry) => canonicalizeJson(entry));

  const out = {};
  for (const key of Object.keys(value).sort()) {
    out[key] = canonicalizeJson(value[key]);
  }
  return out;
}

function canonicalJsonSha256Hex(value) {
  const canonical = canonicalizeJson(value);
  const bytes = Buffer.from(JSON.stringify(canonical), "utf8");
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function normalizeRolepack(pack, label) {
  if (!isPlainObject(pack)) {
    fail("Invalid rolepack root (expected object)", [`Label: ${label}`]);
  }
  if (!Array.isArray(pack.roles)) {
    fail("Invalid rolepack: roles must be an array", [`Label: ${label}`]);
  }

  const roles = [];
  for (const [index, role] of pack.roles.entries()) {
    if (!isPlainObject(role)) {
      fail("Invalid role entry (expected object)", [`Label: ${label}`, `roles[${index}]`]);
    }
    if (typeof role.role_id !== "string" || role.role_id.trim() === "") {
      fail("Invalid role_id (expected non-empty string)", [`Label: ${label}`, `roles[${index}].role_id`]);
    }
    if (!Array.isArray(role.extract_contracts) || !Array.isArray(role.produce_contracts)) {
      fail("Invalid role contracts (expected extract_contracts and produce_contracts arrays)", [
        `Label: ${label}`,
        `role_id=${role.role_id}`,
      ]);
    }
    roles.push(role);
  }

  return { roles };
}

function validateRoleIdsUnique(roles) {
  const seen = new Set();
  const dupes = new Set();
  for (const role of roles) {
    if (seen.has(role.role_id)) dupes.add(role.role_id);
    seen.add(role.role_id);
  }
  if (dupes.size > 0) {
    fail("Duplicate role_id values found", [...dupes].sort());
  }
}

function collectContracts(roles) {
  const contractIdRe = /^ROLE:([^:]+):(X|C):([1-9][0-9]*)$/;
  const contracts = new Map(); // contract_id -> { role_id, kind, version, schema_hash }

  function addContract(role, kindExpected, contract, contractPath) {
    if (!isPlainObject(contract)) {
      fail("Invalid contract entry (expected object)", [`role_id=${role.role_id}`, contractPath]);
    }
    const contractId = contract.contract_id;
    if (typeof contractId !== "string" || contractId.trim() === "") {
      fail("Invalid contract_id (expected non-empty string)", [`role_id=${role.role_id}`, contractPath]);
    }

    const match = contractIdRe.exec(contractId);
    if (!match) {
      fail("Invalid contract_id format (expected ROLE:<role_id>:(X|C):<ver>)", [
        `role_id=${role.role_id}`,
        contractId,
      ]);
    }

    const [, roleIdFromContract, kind, version] = match;
    if (roleIdFromContract !== role.role_id) {
      fail("contract_id role_id mismatch", [`expected role_id=${role.role_id}`, `contract_id=${contractId}`]);
    }
    if (kind !== kindExpected) {
      fail("contract_id kind mismatch", [`expected kind=${kindExpected}`, `contract_id=${contractId}`]);
    }

    if (!("schema_json" in contract) || !isPlainObject(contract.schema_json)) {
      fail("Missing/invalid schema_json for contract (expected object)", [`contract_id=${contractId}`]);
    }

    const schemaHash = canonicalJsonSha256Hex(contract.schema_json);
    if (contracts.has(contractId)) {
      fail("Duplicate contract_id values found", [contractId]);
    }
    contracts.set(contractId, { role_id: role.role_id, kind, version, schema_hash: schemaHash });
  }

  for (const role of roles) {
    for (const [index, contract] of role.extract_contracts.entries()) {
      addContract(role, "X", contract, `extract_contracts[${index}]`);
    }
    for (const [index, contract] of role.produce_contracts.entries()) {
      addContract(role, "C", contract, `produce_contracts[${index}]`);
    }
  }

  return contracts;
}

function main() {
  process.chdir(repoRoot);

  const currentPack = readJsonFromFile(ROLEPACK_PATH);
  const current = normalizeRolepack(currentPack, "current");
  validateRoleIdsUnique(current.roles);
  const currentContracts = collectContracts(current.roles);

  const baselineBytes =
    tryGitShow("main", ROLEPACK_REL_PATH) ??
    tryGitShow("origin/main", ROLEPACK_REL_PATH);

  let baseline = { roles: [] };
  if (baselineBytes !== null) {
    let baselinePack;
    try {
      baselinePack = JSON.parse(baselineBytes);
    } catch (err) {
      fail("Baseline rolepack JSON parse failed", [`From git show`, String(err?.message || err)]);
    }
    baseline = normalizeRolepack(baselinePack, "baseline");
    validateRoleIdsUnique(baseline.roles);
  }

  const baselineRoleIds = new Set(baseline.roles.map((r) => r.role_id));
  const currentRoleIds = new Set(current.roles.map((r) => r.role_id));

  const removedRoleIds = [...baselineRoleIds].filter((roleId) => !currentRoleIds.has(roleId));
  if (removedRoleIds.length > 0) {
    fail("Append-only violation: previously-declared role_id removed", removedRoleIds.sort());
  }

  const baselineContracts = baselineBytes !== null ? collectContracts(baseline.roles) : new Map();
  const removedContractIds = [];
  const driftedContracts = [];

  for (const [contractId, baselineContract] of baselineContracts.entries()) {
    const currentContract = currentContracts.get(contractId);
    if (!currentContract) {
      removedContractIds.push(contractId);
      continue;
    }
    if (currentContract.schema_hash !== baselineContract.schema_hash) {
      driftedContracts.push(
        `${contractId}: expected=${baselineContract.schema_hash} got=${currentContract.schema_hash}`
      );
    }
  }

  if (removedContractIds.length > 0) {
    fail("Append-only violation: previously-declared contract_id removed", removedContractIds.sort());
  }

  if (driftedContracts.length > 0) {
    fail("Contract surface drift: schema_json changed for existing contract_id", driftedContracts.sort());
  }

  console.log("atelier-role-registry-check ok");
}

main();

